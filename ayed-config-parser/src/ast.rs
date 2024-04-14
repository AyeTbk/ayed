#[derive(Debug, Default)]
pub struct Ast<'a> {
    pub top_level_blocks: Vec<Block<'a>>,
}

#[derive(Debug)]
pub enum Block<'a> {
    SelectorBlock(SelectorBlock<'a>),
    MappingBlock(MappingBlock<'a>),
}

impl<'a> Block<'a> {
    pub fn as_selector_block(&self) -> Option<&SelectorBlock<'a>> {
        match self {
            Self::SelectorBlock(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_mapping_block(&self) -> Option<&MappingBlock<'a>> {
        match self {
            Self::MappingBlock(b) => Some(b),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct SelectorBlock<'a> {
    pub state_name: Span<'a>,
    pub pattern: Span<'a>,
    pub children: Vec<Block<'a>>,
}

#[derive(Debug)]
pub struct MappingBlock<'a> {
    pub name: Span<'a>,
    pub entries: Vec<MappingEntry<'a>>,
}

#[derive(Debug)]
pub struct MappingEntry<'a> {
    pub name: Span<'a>,
    pub value: Span<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<'a> {
    pub slice: &'a str,
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(value: &'a str) -> Self {
        Span { slice: value }
    }
}
