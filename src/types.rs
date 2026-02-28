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
