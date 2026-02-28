/// Type representation for the compiler.
///
/// Each variant carries enough information for code generation
/// to determine register sizes, memory layout, and instruction selection.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Char,  // 1 byte, signed
    Short, // 2 bytes, signed
    Int,   // 4 bytes, signed
    Long,  // 8 bytes, signed
}

impl Type {
    /// Determine the common type for binary operations (usual arithmetic conversion).
    /// Integer promotion: char and short are promoted to int.
    /// Then the wider type wins.
    pub fn common_type(a: &Type, b: &Type) -> Type {
        // Integer promotion: char/short → int
        let a = match a {
            Type::Char | Type::Short => Type::Int,
            other => other.clone(),
        };
        let b = match b {
            Type::Char | Type::Short => Type::Int,
            other => other.clone(),
        };
        // Usual arithmetic conversion: wider type wins
        if a.size() >= b.size() { a } else { b }
    }

    /// Returns the size in bytes of this type.
    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Char => 1,
            Type::Short => 2,
            Type::Int => 4,
            Type::Long => 8,
        }
    }

    /// Returns the alignment in bytes of this type.
    pub fn align(&self) -> usize {
        match self {
            Type::Void => 1,
            Type::Char => 1,
            Type::Short => 2,
            Type::Int => 4,
            Type::Long => 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_sizes() {
        assert_eq!(Type::Void.size(), 0);
        assert_eq!(Type::Char.size(), 1);
        assert_eq!(Type::Short.size(), 2);
        assert_eq!(Type::Int.size(), 4);
        assert_eq!(Type::Long.size(), 8);
    }

    #[test]
    fn test_common_type() {
        // Integer promotion: char/short → int
        assert_eq!(Type::common_type(&Type::Char, &Type::Char), Type::Int);
        assert_eq!(Type::common_type(&Type::Short, &Type::Short), Type::Int);
        assert_eq!(Type::common_type(&Type::Char, &Type::Short), Type::Int);

        // Wider type wins
        assert_eq!(Type::common_type(&Type::Int, &Type::Long), Type::Long);
        assert_eq!(Type::common_type(&Type::Char, &Type::Long), Type::Long);
        assert_eq!(Type::common_type(&Type::Int, &Type::Int), Type::Int);
    }
}
