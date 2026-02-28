/// Struct member with name, type, and byte offset.
/// For bit-fields, bit_width and bit_offset specify the sub-byte layout.
#[derive(Debug, Clone, PartialEq)]
pub struct StructMember {
    pub name: String,
    pub ty: Type,
    pub offset: usize,
    /// Bit-field width in bits (0 = normal member, not a bit-field)
    pub bit_width: usize,
    /// Bit offset within the storage unit (0 for normal members)
    pub bit_offset: usize,
}

/// Base type kind.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Void,
    Bool,
    Char,
    Short,
    Int,
    Long,
    Ptr(Box<Type>),
    Array(Box<Type>, usize),
    Struct(Vec<StructMember>),
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
    pub fn bool_type() -> Self { Self { kind: TypeKind::Bool, is_unsigned: true } }
    pub fn char_type() -> Self { Self { kind: TypeKind::Char, is_unsigned: false } }
    pub fn short_type() -> Self { Self { kind: TypeKind::Short, is_unsigned: false } }
    pub fn int_type() -> Self { Self { kind: TypeKind::Int, is_unsigned: false } }
    pub fn long_type() -> Self { Self { kind: TypeKind::Long, is_unsigned: false } }

    // Pointer constructor
    pub fn ptr_to(base: Type) -> Self { Self { kind: TypeKind::Ptr(Box::new(base)), is_unsigned: false } }

    // Array constructor
    pub fn array_of(base: Type, len: usize) -> Self { Self { kind: TypeKind::Array(Box::new(base), len), is_unsigned: false } }

    // Unsigned constructors
    pub fn uchar() -> Self { Self { kind: TypeKind::Char, is_unsigned: true } }
    pub fn ushort() -> Self { Self { kind: TypeKind::Short, is_unsigned: true } }
    pub fn uint() -> Self { Self { kind: TypeKind::Int, is_unsigned: true } }
    pub fn ulong() -> Self { Self { kind: TypeKind::Long, is_unsigned: true } }

    /// Determine the common type for binary operations (usual arithmetic conversion).
    pub fn common_type(a: &Type, b: &Type) -> Type {
        // Integer promotion: char/short → int
        let a = match a.kind {
            TypeKind::Bool | TypeKind::Char | TypeKind::Short => Type::int_type(),
            _ => a.clone(),
        };
        let b = match b.kind {
            TypeKind::Bool | TypeKind::Char | TypeKind::Short => Type::int_type(),
            _ => b.clone(),
        };
        if a.size() >= b.size() { a } else { b }
    }

    /// Returns the size in bytes of this type.
    pub fn size(&self) -> usize {
        match &self.kind {
            TypeKind::Void => 0,
            TypeKind::Bool => 1,
            TypeKind::Char => 1,
            TypeKind::Short => 2,
            TypeKind::Int => 4,
            TypeKind::Long => 8,
            TypeKind::Ptr(_) => 8,
            TypeKind::Array(base, len) => base.size() * len,
            TypeKind::Struct(members) => {
                if members.is_empty() {
                    return 0;
                }
                // Use max of (offset + size) to handle both struct and union layouts
                let raw_size = members.iter()
                    .map(|m| m.offset + m.ty.size())
                    .max()
                    .unwrap_or(0);
                let align = self.align();
                // Align total size to struct/union alignment
                (raw_size + align - 1) & !(align - 1)
            }
        }
    }

    /// Returns the alignment in bytes of this type.
    pub fn align(&self) -> usize {
        match &self.kind {
            TypeKind::Void => 1,
            TypeKind::Bool => 1,
            TypeKind::Char => 1,
            TypeKind::Short => 2,
            TypeKind::Int => 4,
            TypeKind::Long => 8,
            TypeKind::Ptr(_) => 8,
            TypeKind::Array(base, _) => base.align(),
            TypeKind::Struct(members) => {
                members.iter().map(|m| m.ty.align()).max().unwrap_or(1)
            }
        }
    }

    /// Returns the base type of a pointer or array, or None otherwise.
    pub fn base_type(&self) -> Option<&Type> {
        match &self.kind {
            TypeKind::Ptr(base) => Some(base),
            TypeKind::Array(base, _) => Some(base),
            _ => None,
        }
    }

    /// Returns true if this is a pointer or array type.
    pub fn is_pointer(&self) -> bool {
        matches!(self.kind, TypeKind::Ptr(_) | TypeKind::Array(_, _))
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
