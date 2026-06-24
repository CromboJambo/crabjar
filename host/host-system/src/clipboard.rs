/// Clipboard service.
///
/// Provides read/write access to the system clipboard via arboard.
/// Supports text, HTML, and image formats.
use arboard::ImageData;
use crabjar_host_core::event_bus::EventBus;
use std::sync::Arc;

/// Clipboard content type.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardFormat {
    PlainText,
    Html,
    Image,
}

pub struct ClipboardService {
    event_bus: Arc<EventBus>,
}

impl ClipboardService {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self { event_bus }
    }

    /// Read the current clipboard text.
    pub fn get_text(&self) -> Result<String, ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::InitFailed(e.to_string()))?;
        let text = clipboard.get_text()
            .map_err(|e| ClipboardError::ReadFailed(e.to_string()))?;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::ClipboardChanged {
                mime_type: "text/plain".into(),
            },
            "clipboard",
        );
        Ok(text)
    }

    /// Write text to the clipboard.
    pub fn set_text(&self, text: &str) -> Result<(), ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::InitFailed(e.to_string()))?;
        clipboard.set_text(text)
            .map_err(|e| ClipboardError::WriteFailed(e.to_string()))?;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::ClipboardChanged {
                mime_type: "text/plain".into(),
            },
            "clipboard",
        );
        Ok(())
    }

    /// Read HTML content from the clipboard.
    pub fn get_html(&self) -> Result<String, ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::InitFailed(e.to_string()))?;
        let html = clipboard.get()
            .html()
            .map_err(|e| ClipboardError::ReadFailed(e.to_string()))?;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::ClipboardChanged {
                mime_type: "text/html".into(),
            },
            "clipboard",
        );
        Ok(html)
    }

    /// Write HTML content to the clipboard.
    pub fn set_html(&self, html: &str) -> Result<(), ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::InitFailed(e.to_string()))?;
        clipboard.set_html(html, None::<&str>)
            .map_err(|e| ClipboardError::WriteFailed(e.to_string()))?;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::ClipboardChanged {
                mime_type: "text/html".into(),
            },
            "clipboard",
        );
        Ok(())
    }

    /// Read image from the clipboard.
    pub fn get_image(&self) -> Result<ImageData<'_>, ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::InitFailed(e.to_string()))?;
        let image = clipboard.get_image()
            .map_err(|e| ClipboardError::ReadFailed(e.to_string()))?;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::ClipboardChanged {
                mime_type: "image/png".into(),
            },
            "clipboard",
        );
        Ok(image)
    }

    /// Write image to the clipboard.
    pub fn set_image(&self, image: ImageData) -> Result<(), ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| ClipboardError::InitFailed(e.to_string()))?;
        clipboard.set_image(image)
            .map_err(|e| ClipboardError::WriteFailed(e.to_string()))?;
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::ClipboardChanged {
                mime_type: "image/png".into(),
            },
            "clipboard",
        );
        Ok(())
    }

    /// Check if the clipboard has text content.
    pub fn has_text(&self) -> bool {
        arboard::Clipboard::new()
            .map(|mut cb| cb.get_text().is_ok())
            .unwrap_or(false)
    }

    /// Check if the clipboard has image content.
    pub fn has_image(&self) -> bool {
        arboard::Clipboard::new()
            .map(|mut cb| cb.get_image().is_ok())
            .unwrap_or(false)
    }
}

/// Clipboard errors.
#[derive(thiserror::Error, Debug)]
pub enum ClipboardError {
    #[error("failed to initialize clipboard: {0}")]
    InitFailed(String),
    #[error("failed to read clipboard: {0}")]
    ReadFailed(String),
    #[error("failed to write clipboard: {0}")]
    WriteFailed(String),
}
