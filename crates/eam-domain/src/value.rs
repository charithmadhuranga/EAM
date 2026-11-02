//! Entity value types

use serde::{Deserialize, Serialize};

/// Entity value - tagged union for all possible entity states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum EntityValue {
    /// Numeric value (temperature, humidity, etc.)
    Number(f64),
    
    /// Boolean value (on/off, open/closed, etc.)
    Bool(bool),
    
    /// Text value
    Text(String),
    
    /// JSON value (complex state)
    Json(serde_json::Value),
    
    /// Unknown or unavailable state
    Unknown,
}

impl EntityValue {
    /// Create a numeric value
    pub fn number(value: f64) -> Self {
        Self::Number(value)
    }
    
    /// Create a boolean value
    pub fn bool(value: bool) -> Self {
        Self::Bool(value)
    }
    
    /// Create a text value
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
    
    /// Check if value is numeric
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }
    
    /// Check if value is boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }
    
    /// Get numeric value
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Get boolean value
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
    
    /// Get text value
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

impl Default for EntityValue {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for EntityValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Text(s) => write!(f, "{}", s),
            Self::Json(v) => write!(f, "{}", v),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_value() {
        let value = EntityValue::number(23.5);
        
        assert!(value.is_number());
        assert!(!value.is_bool());
        assert_eq!(value.as_number(), Some(23.5));
        assert_eq!(value.as_bool(), None);
    }

    #[test]
    fn test_bool_value() {
        let value = EntityValue::bool(true);
        
        assert!(value.is_bool());
        assert!(!value.is_number());
        assert_eq!(value.as_bool(), Some(true));
        assert_eq!(value.as_number(), None);
    }

    #[test]
    fn test_text_value() {
        let value = EntityValue::text("active");
        
        assert_eq!(value.as_text(), Some("active"));
        assert!(value.as_number().is_none());
    }
}
