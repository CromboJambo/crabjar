use serde::{Deserialize, Serialize};

/// GGUF file format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgufVersion {
    V1,
    V2,
    V3,
}

impl GgufVersion {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            3 => Some(Self::V3),
            _ => None,
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
        }
    }
}

/// GGUF key-value value type.
///
/// Maps to GGUF spec value types:
/// UINT8=0, INT8=1, UINT16=2, INT16=3, UINT32=4, INT32=5,
/// UINT64=6, INT64=7, STRING=8, FLOAT32=9, FLOAT64=10,
/// BOOL=11, ARRAY=12, BFLOAT16=15
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GgufValueType {
    Uint8,
    Int8,
    Uint16,
    Int16,
    Uint32,
    Int32,
    Uint64,
    Int64,
    String,
    Float32,
    Float64,
    Bool,
    Array,
    Bfloat16,
}

impl GgufValueType {
    pub const fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Uint8),
            1 => Some(Self::Int8),
            2 => Some(Self::Uint16),
            3 => Some(Self::Int16),
            4 => Some(Self::Uint32),
            5 => Some(Self::Int32),
            6 => Some(Self::Uint64),
            7 => Some(Self::Int64),
            8 => Some(Self::String),
            9 => Some(Self::Float32),
            10 => Some(Self::Float64),
            11 => Some(Self::Bool),
            12 => Some(Self::Array),
            15 => Some(Self::Bfloat16),
            _ => None,
        }
    }

    pub const fn to_u32(self) -> u32 {
        match self {
            Self::Uint8 => 0,
            Self::Int8 => 1,
            Self::Uint16 => 2,
            Self::Int16 => 3,
            Self::Uint32 => 4,
            Self::Int32 => 5,
            Self::Uint64 => 6,
            Self::Int64 => 7,
            Self::String => 8,
            Self::Float32 => 9,
            Self::Float64 => 10,
            Self::Bool => 11,
            Self::Array => 12,
            Self::Bfloat16 => 15,
        }
    }

    pub fn is_array(self) -> bool {
        self == Self::Array
    }

    pub fn element_size(self) -> Option<usize> {
        match self {
            Self::Uint8 | Self::Int8 | Self::Bool => Some(1),
            Self::Uint16 | Self::Int16 | Self::Bfloat16 => Some(2),
            Self::Uint32 | Self::Int32 => Some(4),
            Self::Uint64 | Self::Int64 | Self::Float64 => Some(8),
            Self::Float32 => Some(4),
            Self::String | Self::Array => None,
        }
    }
}

/// A single key-value pair from the GGUF header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufKvPair {
    pub key: String,
    pub value_type: GgufValueType,
    pub value: GgufKvValue,
}

impl GgufKvPair {
    /// Total byte size of this KV pair in the GGUF file (key_len + key + type + value).
    pub fn raw_byte_size(&self) -> usize {
        let key_bytes = self.key.len();
        let value_bytes = match &self.value {
            GgufKvValue::Uint8(..)
            | GgufKvValue::Int8(..)
            | GgufKvValue::Bool(..) => 1,
            GgufKvValue::Uint16(..) | GgufKvValue::Int16(..) => 2,
            GgufKvValue::Uint32(..)
            | GgufKvValue::Int32(..)
            | GgufKvValue::Float32(..)
            | GgufKvValue::Bfloat16(..) => 4,
            GgufKvValue::Uint64(..) | GgufKvValue::Int64(..) | GgufKvValue::Float64(..) => 8,
            GgufKvValue::String(s) => 8 + s.len(),
            GgufKvValue::Array(arr) => {
                let elem_size = match arr.first().map(|v| v.value_type()) {
                    Some(GgufValueType::Uint8 | GgufValueType::Int8 | GgufValueType::Bool) => 1,
                    Some(GgufValueType::Uint16 | GgufValueType::Int16) => 2,
                    Some(GgufValueType::Uint32 | GgufValueType::Int32 | GgufValueType::Float32) => 4,
                    Some(GgufValueType::Uint64 | GgufValueType::Int64 | GgufValueType::Float64) => 8,
                    Some(GgufValueType::String) => {
                        return arr.iter().map(|v| match v {
                            GgufKvValue::String(s) => 8 + s.len(),
                            _ => 0,
                        }).sum::<usize>() + 4 + 8;
                    }
                    _ => 4,
                };
                4 + 8 + arr.len() * elem_size
            }
        };
        8 + key_bytes + 4 + value_bytes
    }
}

/// GGUF tensor data type (stored on disk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum GgufDtype {
    F32,
    F16,
    Q4_0,
    Q4_1,
    Q5_0,
    Q5_1,
    Q8_0,
    Q8_1,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
    Q8_K,
    I8,
    I16,
    I32,
    I64,
    F64,
    BF16,
    Q1_K,
    Q4_K_M,
    Q5_K_M,
    Q6_K_S,
    Q8_K_M,
    Q2_K_S,
    Q3_K_S,
    Q4_K_S,
    Q5_K_S,
    Q2_K_M,
    Unknown(u32),
}

impl GgufDtype {
    pub const fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::F32,
            1 => Self::F16,
            2 => Self::Q4_0,
            3 => Self::Q4_1,
            4 => Self::Q5_0,
            5 => Self::Q5_1,
            6 => Self::Q8_0,
            7 => Self::Q8_1,
            8 => Self::Q2_K,
            9 => Self::Q3_K,
            10 => Self::Q4_K,
            11 => Self::Q5_K,
            12 => Self::Q6_K,
            13 => Self::Q8_K,
            14 => Self::I8,
            15 => Self::I16,
            16 => Self::I32,
            17 => Self::I64,
            18 => Self::F64,
            19 => Self::BF16,
            20 => Self::Q1_K,
            21 => Self::Q4_K_M,
            22 => Self::Q5_K_M,
            23 => Self::Q6_K_S,
            24 => Self::Q8_K_M,
            25 => Self::Q2_K_S,
            26 => Self::Q3_K_S,
            27 => Self::Q4_K_S,
            28 => Self::Q5_K_S,
            29 => Self::Q2_K_M,
            _ => Self::Unknown(v),
        }
    }

    pub const fn to_u32(self) -> u32 {
        match self {
            Self::F32 => 0,
            Self::F16 => 1,
            Self::Q4_0 => 2,
            Self::Q4_1 => 3,
            Self::Q5_0 => 4,
            Self::Q5_1 => 5,
            Self::Q8_0 => 6,
            Self::Q8_1 => 7,
            Self::Q2_K => 8,
            Self::Q3_K => 9,
            Self::Q4_K => 10,
            Self::Q5_K => 11,
            Self::Q6_K => 12,
            Self::Q8_K => 13,
            Self::I8 => 14,
            Self::I16 => 15,
            Self::I32 => 16,
            Self::I64 => 17,
            Self::F64 => 18,
            Self::BF16 => 19,
            Self::Q1_K => 20,
            Self::Q4_K_M => 21,
            Self::Q5_K_M => 22,
            Self::Q6_K_S => 23,
            Self::Q8_K_M => 24,
            Self::Q2_K_S => 25,
            Self::Q3_K_S => 26,
            Self::Q4_K_S => 27,
            Self::Q5_K_S => 28,
            Self::Q2_K_M => 29,
            Self::Unknown(v) => v,
        }
    }

    pub const fn is_quantized(self) -> bool {
        matches!(
            self,
            Self::Q4_0
                | Self::Q4_1
                | Self::Q5_0
                | Self::Q5_1
                | Self::Q8_0
                | Self::Q8_1
                | Self::Q2_K
                | Self::Q3_K
                | Self::Q4_K
                | Self::Q5_K
                | Self::Q6_K
                | Self::Q8_K
                | Self::Q1_K
                | Self::Q4_K_M
                | Self::Q5_K_M
                | Self::Q6_K_S
                | Self::Q8_K_M
                | Self::Q2_K_S
                | Self::Q3_K_S
                | Self::Q4_K_S
                | Self::Q5_K_S
                | Self::Q2_K_M
        )
    }

    pub const fn bytes_per_element(self) -> usize {
        match self {
            Self::F32 => 4,
            Self::F16 => 2,
            Self::Q8_0 | Self::Q8_1 => 2,
            Self::I8 => 1,
            Self::I16 => 2,
            Self::I32 => 4,
            Self::I64 => 8,
            Self::F64 => 8,
            Self::BF16 => 2,
            Self::Q4_0 | Self::Q4_1 | Self::Q1_K | Self::Q5_0 | Self::Q5_1 | Self::Q4_K_M => 0,
            Self::Q2_K | Self::Q3_K | Self::Q4_K | Self::Q5_K | Self::Q5_K_S | Self::Q5_K_M | Self::Q6_K | Self::Q6_K_S | Self::Q8_K | Self::Q8_K_M | Self::Q2_K_M | Self::Q2_K_S | Self::Q3_K_S | Self::Q4_K_S => 0,
            Self::Unknown(_) => 0,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::F32 => "F32",
            Self::F16 => "F16",
            Self::Q4_0 => "Q4_0",
            Self::Q4_1 => "Q4_1",
            Self::Q5_0 => "Q5_0",
            Self::Q5_1 => "Q5_1",
            Self::Q8_0 => "Q8_0",
            Self::Q8_1 => "Q8_1",
            Self::Q2_K => "Q2_K",
            Self::Q3_K => "Q3_K",
            Self::Q4_K => "Q4_K",
            Self::Q5_K => "Q5_K",
            Self::Q6_K => "Q6_K",
            Self::Q8_K => "Q8_K",
            Self::I8 => "I8",
            Self::I16 => "I16",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::F64 => "F64",
            Self::BF16 => "BF16",
            Self::Q1_K => "Q1_K",
            Self::Q4_K_M => "Q4_K_M",
            Self::Q5_K_M => "Q5_K_M",
            Self::Q6_K_S => "Q6_K_S",
            Self::Q8_K_M => "Q8_K_M",
            Self::Q2_K_S => "Q2_K_S",
            Self::Q3_K_S => "Q3_K_S",
            Self::Q4_K_S => "Q4_K_S",
            Self::Q5_K_S => "Q5_K_S",
            Self::Q2_K_M => "Q2_K_M",
            Self::Unknown(_) => "unknown",
        }
    }
}

/// A single tensor's metadata (name, shape, dtype, offset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufTensorInfo {
    pub name: String,
    pub shape: Vec<u64>,
    pub offset: u64,
    pub dtype: u32,
}

impl GgufTensorInfo {
    pub fn element_count(&self) -> u64 {
        self.shape.iter().product()
    }

    pub fn ndims(&self) -> u32 {
        self.shape.len() as u32
    }

    /// Compute the actual stored byte size on disk.
    ///
    /// For quantized tensors, this is much smaller than element_count * 2 (F16).
    pub fn stored_size(&self) -> u64 {
        let n = self.element_count();
        let dtype = GgufDtype::from_u32(self.dtype);
        match dtype {
            GgufDtype::F32 => n * 4,
            GgufDtype::F16 | GgufDtype::BF16 => n * 2,
            GgufDtype::Q8_0 => {
                let full_blocks = n / 256;
                let remaining = n % 256;
                full_blocks * 258 + if remaining > 0 { 2 + remaining } else { 0 }
            }
            GgufDtype::Q8_1 => n / 2 + 128 + 128,
            GgufDtype::Q4_0 => {
                let full_blocks = n / 32;
                let remaining = n % 32;
                full_blocks * 20 + if remaining > 0 { 4 + remaining.div_ceil(2) } else { 0 }
            }
            GgufDtype::Q4_1 => {
                let full_blocks = n / 32;
                let remaining = n % 32;
                full_blocks * 20 + if remaining > 0 { 4 + remaining.div_ceil(2) } else { 0 }
            }
            GgufDtype::Q5_0 => n / 2 + 32 + 16,
            GgufDtype::Q5_1 => n / 2 + 64 + 16,
            GgufDtype::Q2_K => n / 4 + n * 6 / 32 + 8,
            GgufDtype::Q3_K => n / 8 + n * 6 / 32 + 16,
            GgufDtype::Q4_K => n / 4 + n * 6 / 32 + 16 + 32,
            GgufDtype::Q5_K => n / 4 + n * 6 / 32 + 16 + 32 + 16,
            GgufDtype::Q6_K => n / 2 + n / 4 + 256,
            GgufDtype::Q8_K => n / 2 + n * 6 / 32 + 256,
            GgufDtype::Q1_K => n / 8 + n * 6 / 32 + 96,
            GgufDtype::Q4_K_M | GgufDtype::Q5_K_M | GgufDtype::Q8_K_M => n / 4 + n * 6 / 32 + 48,
            GgufDtype::Q2_K_S | GgufDtype::Q3_K_S | GgufDtype::Q4_K_S | GgufDtype::Q5_K_S | GgufDtype::Q6_K_S | GgufDtype::Q2_K_M => n / 4 + n * 6 / 32 + 24,
            GgufDtype::I8 => n,
            GgufDtype::I16 => n * 2,
            GgufDtype::I32 => n * 4,
            GgufDtype::I64 => n * 8,
            GgufDtype::F64 => n * 8,
            GgufDtype::Unknown(_) => n * 2,
        }
    }

    /// Total byte size of this tensor info in the GGUF file (name_len + name + dims + shape + dtype + offset).
    pub fn raw_byte_size(&self) -> usize {
        8 + self.name.len() + 4 + (self.shape.len() * 8) + 4 + 8
    }
}

/// Parsed key-value value (runtime representation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GgufKvValue {
    Uint8(u8),
    Int8(i8),
    Uint16(u16),
    Int16(i16),
    Uint32(u32),
    Int32(i32),
    Uint64(u64),
    Int64(i64),
    String(String),
    Float32(f32),
    Float64(f64),
    Bool(bool),
    Array(Vec<GgufKvValue>),
    Bfloat16(f32),
}

impl GgufKvValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            GgufKvValue::Uint8(v) => Some(*v as u64),
            GgufKvValue::Uint16(v) => Some(*v as u64),
            GgufKvValue::Uint32(v) => Some(*v as u64),
            GgufKvValue::Uint64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            GgufKvValue::Int8(v) => Some(*v as i64),
            GgufKvValue::Int16(v) => Some(*v as i64),
            GgufKvValue::Int32(v) => Some(*v as i64),
            GgufKvValue::Int64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            GgufKvValue::Uint8(v) => Some(*v as u32),
            GgufKvValue::Uint16(v) => Some(*v as u32),
            GgufKvValue::Uint32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            GgufKvValue::Int8(v) => Some(*v as i32),
            GgufKvValue::Int16(v) => Some(*v as i32),
            GgufKvValue::Int32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            GgufKvValue::Float32(v) => Some(*v),
            GgufKvValue::Float64(v) => Some(*v as f32),
            GgufKvValue::Bfloat16(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            GgufKvValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            GgufKvValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<GgufKvValue>> {
        match self {
            GgufKvValue::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn value_type(&self) -> GgufValueType {
        match self {
            GgufKvValue::Uint8(..) => GgufValueType::Uint8,
            GgufKvValue::Int8(..) => GgufValueType::Int8,
            GgufKvValue::Uint16(..) => GgufValueType::Uint16,
            GgufKvValue::Int16(..) => GgufValueType::Int16,
            GgufKvValue::Uint32(..) => GgufValueType::Uint32,
            GgufKvValue::Int32(..) => GgufValueType::Int32,
            GgufKvValue::Uint64(..) => GgufValueType::Uint64,
            GgufKvValue::Int64(..) => GgufValueType::Int64,
            GgufKvValue::String(..) => GgufValueType::String,
            GgufKvValue::Float32(..) => GgufValueType::Float32,
            GgufKvValue::Float64(..) => GgufValueType::Float64,
            GgufKvValue::Bool(..) => GgufValueType::Bool,
            GgufKvValue::Array(..) => GgufValueType::Array,
            GgufKvValue::Bfloat16(..) => GgufValueType::Bfloat16,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            GgufKvValue::Uint8(_) => "u8",
            GgufKvValue::Int8(_) => "i8",
            GgufKvValue::Uint16(_) => "u16",
            GgufKvValue::Int16(_) => "i16",
            GgufKvValue::Uint32(_) => "u32",
            GgufKvValue::Int32(_) => "i32",
            GgufKvValue::Uint64(_) => "u64",
            GgufKvValue::Int64(_) => "i64",
            GgufKvValue::String(_) => "str",
            GgufKvValue::Float32(_) => "f32",
            GgufKvValue::Float64(_) => "f64",
            GgufKvValue::Bool(_) => "bool",
            GgufKvValue::Array(_) => "array",
            GgufKvValue::Bfloat16(_) => "bf16",
        }
    }
}

/// Parsed GGUF header (everything before tensor data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufHeader {
    pub version: u32,
    pub kv_pairs: Vec<GgufKvPair>,
    pub tensors: Vec<GgufTensorInfo>,
    pub data_alignment: Option<u64>,
    pub data_section_start: u64,
}

impl GgufHeader {
    pub fn get_kv<T: From<GgufKvValue>>(&self, key: &str) -> Option<T> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .map(|p| T::from(p.value.clone()))
    }

    pub fn get_kv_str(&self, key: &str) -> Option<&str> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_str())
    }

    pub fn get_kv_u32(&self, key: &str) -> Option<u32> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_u32())
    }

    pub fn get_kv_i32(&self, key: &str) -> Option<i32> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_i32())
    }

    pub fn get_kv_u64(&self, key: &str) -> Option<u64> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_u64())
    }

    pub fn get_kv_f32(&self, key: &str) -> Option<f32> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_f32())
    }

    pub fn get_kv_bool(&self, key: &str) -> Option<bool> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_bool())
    }

    pub fn get_kv_array(&self, key: &str) -> Option<&Vec<GgufKvValue>> {
        self.kv_pairs
            .iter()
            .find(|p| p.key == key)
            .and_then(|p| p.value.as_array())
    }

    pub fn to_config_map(&self) -> std::collections::HashMap<String, GgufKvValue> {
        self.kv_pairs
            .iter()
            .map(|p| (p.key.clone(), p.value.clone()))
            .collect()
    }

    /// Extract architecture name (e.g., "llama", "mistral", "qwen2").
    pub fn architecture(&self) -> Option<&str> {
        self.get_kv_str("general.architecture")
            .or_else(|| self.get_kv_str("arch"))
    }

    /// Extract file type string (e.g., "F16", "Q4_0", "Q8_0").
    pub fn file_type(&self) -> Option<String> {
        self.get_kv_str("general.file_type")
            .map(|s| s.to_string())
            .or_else(|| self.get_kv_str("ft").map(|s| s.to_string()))
            .or_else(|| self.get_kv_u32("general.file_type").map(|v| v.to_string()))
            .or_else(|| self.get_kv_u32("ft").map(|v| v.to_string()))
    }

    /// Extract context length (n_ctx).
    pub fn context_length(&self) -> Option<u32> {
        self.get_kv_u32("llama.context_length")
            .or_else(|| self.get_kv_u32("context_length"))
            .or_else(|| self.get_kv_u32("n_ctx"))
    }

    /// Extract embedding/vector dimension.
    pub fn embedding_length(&self) -> Option<u32> {
        self.get_kv_u32("llama.embedding_length")
            .or_else(|| self.get_kv_u32("embedding_length"))
            .or_else(|| self.get_kv_u32("n_embd"))
    }

    /// Extract block count (number of layers).
    pub fn block_count(&self) -> Option<u32> {
        self.get_kv_u32("llama.block_count")
            .or_else(|| self.get_kv_u32("block_count"))
            .or_else(|| self.get_kv_u32("n_layer"))
    }

    /// Extract attention head count.
    pub fn attention_head_count(&self) -> Option<u32> {
        self.get_kv_u32("llama.attention.head_count")
            .or_else(|| self.get_kv_u32("attention.head_count"))
            .or_else(|| self.get_kv_u32("n_head"))
    }

    /// Extract attention head count for KV (QKV projection).
    pub fn attention_head_count_kv(&self) -> Option<u32> {
        self.get_kv_u32("llama.attention.head_count_kv")
            .or_else(|| self.get_kv_u32("attention.head_count_kv"))
    }

    /// Extract rope dimension count.
    pub fn rope_dimension_count(&self) -> Option<i32> {
        self.get_kv_i32("llama.rope.dimension_count")
            .or_else(|| self.get_kv_i32("rope.dimension_count"))
            .or_else(|| self.get_kv_i32("rope_dim"))
    }

    /// Extract feed-forward dimension.
    pub fn feed_forward_length(&self) -> Option<u32> {
        self.get_kv_u32("llama.feed_forward_length")
            .or_else(|| self.get_kv_u32("feed_forward_length"))
            .or_else(|| self.get_kv_u32("n_ff"))
    }

    /// Extract rope scaling parameters.
    pub fn rope_scaling(&self) -> Option<&Vec<GgufKvValue>> {
        self.get_kv_array("rope.scaling")
    }

    /// Extract rope scaling type (e.g., "linear", "yarn").
    pub fn rope_scaling_type(&self) -> Option<&str> {
        self.get_kv_str("rope.scaling.type")
            .or_else(|| self.get_kv_str("rope_type"))
    }

    /// Extract token embedding length (vocabulary size).
    pub fn vocab_size(&self) -> Option<u32> {
        self.get_kv_u32("tokenizer.ggml.tokens")
            .or_else(|| self.get_kv_u32("vocab_size"))
            .or_else(|| self.get_kv_u32("n_vocab"))
    }

    /// Extract normalization epsilon.
    pub fn normalization_epsilon(&self) -> Option<f32> {
        self.get_kv_f32("llama.attention.layer_norm_rms_epsilon")
            .or_else(|| self.get_kv_f32("attention.layer_norm_epsilon"))
            .or_else(|| self.get_kv_f32("layer_norm_epsilon"))
            .or_else(|| self.get_kv_f32("rms_norm_eps"))
    }

    /// Extract quantization description if present.
    pub fn quantization_description(&self) -> Option<&str> {
        self.get_kv_str("general.quantization_version")
            .or_else(|| self.get_kv_str("quantization"))
    }

    /// Get tensor by name.
    pub fn get_tensor(&self, name: &str) -> Option<&GgufTensorInfo> {
        self.tensors.iter().find(|t| t.name == name)
    }

    /// Check if a tensor exists.
    pub fn has_tensor(&self, name: &str) -> bool {
        self.tensors.iter().any(|t| t.name == name)
    }

    /// Total tensor data size in bytes (sum of all tensor sizes assuming f32).
    /// Actual size depends on quantization — this is an upper bound.
    pub fn total_tensor_bytes_f32(&self) -> u64 {
        self.tensors.iter().map(|t| t.element_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gguf_version_from_u32() {
        assert_eq!(GgufVersion::from_u32(1), Some(GgufVersion::V1));
        assert_eq!(GgufVersion::from_u32(2), Some(GgufVersion::V2));
        assert_eq!(GgufVersion::from_u32(3), Some(GgufVersion::V3));
        assert_eq!(GgufVersion::from_u32(4), None);
    }

    #[test]
    fn test_gguf_version_to_u32() {
        assert_eq!(GgufVersion::V1.to_u32(), 1);
        assert_eq!(GgufVersion::V2.to_u32(), 2);
        assert_eq!(GgufVersion::V3.to_u32(), 3);
    }

    #[test]
    fn test_value_type_from_u32() {
        assert_eq!(GgufValueType::from_u32(0), Some(GgufValueType::Uint8));
        assert_eq!(GgufValueType::from_u32(12), Some(GgufValueType::Array));
        assert_eq!(GgufValueType::from_u32(15), Some(GgufValueType::Bfloat16));
        assert_eq!(GgufValueType::from_u32(14), None); // 14 is reserved
    }

    #[test]
    fn test_value_type_element_size() {
        assert_eq!(GgufValueType::Uint8.element_size(), Some(1));
        assert_eq!(GgufValueType::Float32.element_size(), Some(4));
        assert_eq!(GgufValueType::String.element_size(), None);
        assert_eq!(GgufValueType::Array.element_size(), None);
    }

    #[test]
    fn test_tensor_info_element_count() {
        let info = GgufTensorInfo {
            name: "test".to_string(),
            shape: vec![2, 3, 4],
            offset: 0,
            dtype: 0,
        };
        assert_eq!(info.element_count(), 24);
        assert_eq!(info.ndims(), 3);
    }

    #[test]
    fn test_gguf_header_helpers() {
        let header = GgufHeader {
            version: 3,
            kv_pairs: vec![
                GgufKvPair {
                    key: "general.architecture".to_string(),
                    value_type: GgufValueType::String,
                    value: GgufKvValue::String("llama".to_string()),
                },
                GgufKvPair {
                    key: "llama.context_length".to_string(),
                    value_type: GgufValueType::Uint32,
                    value: GgufKvValue::Uint32(4096),
                },
                GgufKvPair {
                    key: "llama.embedding_length".to_string(),
                    value_type: GgufValueType::Uint32,
                    value: GgufKvValue::Uint32(4096),
                },
                GgufKvPair {
                    key: "llama.attention.layer_norm_rms_epsilon".to_string(),
                    value_type: GgufValueType::Float32,
                    value: GgufKvValue::Float32(1e-5),
                },
            ],
            tensors: vec![
                GgufTensorInfo {
                    name: "token_embd.weight".to_string(),
                    shape: vec![4096],
                    offset: 0,
                    dtype: 1,
                },
            ],
            data_alignment: Some(32),
            data_section_start: 0,
        };
    }
}
