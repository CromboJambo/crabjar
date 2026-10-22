use pesti_structural_tokenizer::semantic::analyze_ownership;

fn main() {
    let source = "fn modify_data(data: &mut Vec<i32>) {\n    data.push(42);\n}";
    let result = analyze_ownership(source);
    println!("Pesti ownership annotations: {:?}", result);
    
    // Also test with simpler pattern
    let source2 = "let x = &mut vec![1, 2, 3];";
    let result2 = analyze_ownership(source2);
    println!("Pesti ownership annotations (simple): {:?}", result2);
}
