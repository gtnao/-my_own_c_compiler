/// Base type kind.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Void,
    Char,
    Short,
    Int,
    Long,
}

/// Type representation with signedness.
#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub is_unsigned: bool,
}

impl Type {
    // Signed constructors
    pub fn void() -> Self { Self { kind: TypeKind::Void, is_unsigned: false } }
    pub fn char_type() -> Self { Self { kind: TypeKind::Char, is_unsigned: false } }
    pub fn short_type() -> Self { Self { kind: TypeKind::Short, is_unsigned: false } }
    pub fn int_type() -> Self { Self { kind: TypeKind::Int, is_unsigned: false } }
    pub fn long_type() -> Self { Self { kind: TypeKind::Long, is_unsigned: false } }

    // Unsigned constructors
    pub fn uchar() -> Self { Self { kind: TypeKind::Char, is_unsigned: true } }
    pub fn ushort() -> Self { Self { kind: TypeKind::Short, is_unsigned: true } }
    pub fn uint() -> Self { Self { kind: TypeKind::Int, is_unsigned: true } }
    pub fn ulong() -> Self { Self { kind: TypeKind::Long, is_unsigned: true } }

    /// Determine the common type for binary operations (usual arithmetic conversion).
    pub fn common_type(a: &Type, b: &Type) -> Type {
        // Integer promotion: char/short → int
        let a = match a.kind {
            TypeKind::Char | TypeKind::Short => Type::int_type(),
            _ => a.clone(),
        };
        let b = match b.kind {
            TypeKind::Char | TypeKind::Short => Type::int_type(),
            _ => b.clone(),
        };
        if a.size() >= b.size() { a } else { b }
    }

    /// Returns the size in bytes of this type.
    pub fn size(&self) -> usize {
        match self.kind {
            TypeKind::Void => 0,
            TypeKind::Char => 1,
            TypeKind::Short => 2,
            TypeKind::Int => 4,
            TypeKind::Long => 8,
        }
    }

    /// Returns the alignment in bytes of this type.
    pub fn align(&self) -> usize {
        match self.kind {
            TypeKind::Void => 1,
            TypeKind::Char => 1,
            TypeKind::Short => 2,
            TypeKind::Int => 4,
            TypeKind::Long => 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_sizes() {
        assert_eq!(Type::void().size(), 0);
        assert_eq!(Type::char_type().size(), 1);
        assert_eq!(Type::short_type().size(), 2);
        assert_eq!(Type::int_type().size(), 4);
        assert_eq!(Type::long_type().size(), 8);
        // Unsigned types have same sizes
        assert_eq!(Type::uchar().size(), 1);
        assert_eq!(Type::uint().size(), 4);
    }

    #[test]
    fn test_common_type() {
        assert_eq!(Type::common_type(&Type::char_type(), &Type::char_type()), Type::int_type());
        assert_eq!(Type::common_type(&Type::short_type(), &Type::short_type()), Type::int_type());
        assert_eq!(Type::common_type(&Type::int_type(), &Type::long_type()), Type::long_type());
        assert_eq!(Type::common_type(&Type::char_type(), &Type::long_type()), Type::long_type());
        assert_eq!(Type::common_type(&Type::int_type(), &Type::int_type()), Type::int_type());
    }
}
