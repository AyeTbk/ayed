#[derive(Debug, Default)]
pub struct Ast<'a> {
    pub top_level_blocks: Vec<Block<'a>>,
}

#[derive(Debug)]
pub struct Block<'a> {
    pub is_override: bool,
    pub kind: BlockKind<'a>,
}

#[derive(Debug)]
pub enum BlockKind<'a> {
    Selector(SelectorBlock<'a>),
    Mapping(MappingBlock<'a>),
    Mixin(MixinBlock<'a>),
    Use(Span<'a>),
}

impl<'a> BlockKind<'a> {
    pub fn as_selector_block(&self) -> Option<&SelectorBlock<'a>> {
        match self {
            Self::Selector(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_mapping_block(&self) -> Option<&MappingBlock<'a>> {
        match self {
            Self::Mapping(b) => Some(b),
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
    pub values: Vec<Span<'a>>,
}

#[derive(Debug)]
pub struct MixinBlock<'a> {
    pub name: Span<'a>,
    pub children: Vec<Block<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<'a> {
    pub slice: &'a str,
}

impl<'a> Span<'a> {
    pub fn to_string(&self) -> String {
        self.slice.to_string()
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(value: &'a str) -> Self {
        Span { slice: value }
    }
}
