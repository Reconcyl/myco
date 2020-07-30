use super::ui::Color;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Category {
    Special,
    Wall,
    Calculation,
    Control,
    Cursor,
    Selection,
    Memory,
}

impl Category {
    pub fn color(self) -> Color {
        match self {
            Self::Special     => Color::Gray,
            Self::Wall        => Color::LightGray,
            Self::Calculation => Color::LightGreen,
            Self::Control     => Color::LightMagenta,
            Self::Cursor      => Color::LightCyan,
            Self::Selection   => Color::LightRed,
            Self::Memory      => Color::LightBlue,
        }
    }
    pub fn color_rgb(self) -> [u8; 3] {
        match self {
            Self::Special     => [0x30, 0x30, 0x30],
            Self::Wall        => [0x8a, 0x8a, 0x8a],
            Self::Calculation => [0x8e, 0xcd, 0x00],
            Self::Control     => [0xc4, 0x6a, 0xe1],
            Self::Cursor      => [0x00, 0xd4, 0xd9],
            Self::Selection   => [0xe1, 0x00, 0x03],
            Self::Memory      => [0x74, 0xa4, 0xdc],
        }
    }
    pub const PALETTE: [u8; 3 * 7] = [
        0x30, 0x30, 0x30,
        0x8a, 0x8a, 0x8a,
        0x8e, 0xcd, 0x00,
        0xc4, 0x6a, 0xe1,
        0x00, 0xd4, 0xd9,
        0xe1, 0x00, 0x03,
        0x74, 0xa4, 0xdc,
    ];
}

macro_rules! gen_variant {
    (
        $enum_name:ident
        (
            const $array_name:ident,
            const $symbol_array_name:ident
        )
        $($variant:ident $symbol:literal $category:ident)*
    ) => {
        #[repr(u8)]
        #[derive(Clone, Copy)]
        pub enum $enum_name {
            $($variant,)*
        }
        impl $enum_name {
            pub fn category(self) -> Category {
                match self {
                    $(Self::$variant => Category::$category,)*
                }
            }
        }
        static $array_name: &[$enum_name] = &[$($enum_name::$variant,)*];
        static $symbol_array_name: &[&str] = &[$($symbol,)*];
        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", match self {
                    $(Self::$variant => $symbol,)*
                })?;
                Ok(())
            }
        }
    }
}

// Create the `Instruction` enum with methods that map each instruction
// to its symbol or to its category.
// Create a constant array of instructions in order and a static array
// of instruction symbols which can be used to perform lookup.
gen_variant! { Instruction (const INSTRUCTIONS, const INSTRUCTION_SYMBOLS)
    Halt        "@@"  Special
    Nop         ".."  Special
    FlagFork    "-="  Special
    CursorFork  "m="  Special

    Wall  "##"  Wall

    ZeroA     "0a"  Calculation
    ZeroB     "0b"  Calculation
    CopyA     "ba"  Calculation
    CopyB     "ab"  Calculation
    SwapAB    "::"  Calculation
    SumA      "a+"  Calculation
    SumB      "b+"  Calculation
    NegateA   "a-"  Calculation
    NegateB   "b-"  Calculation
    IncA      "+a"  Calculation
    IncB      "+b"  Calculation
    DecA      "-a"  Calculation
    DecB      "-b"  Calculation
    MulA      "a*"  Calculation
    MulB      "b*"  Calculation
    DoubleA   "aa"  Calculation
    DoubleB   "bb"  Calculation
    HalveA    "a/"  Calculation
    HalveB    "b/"  Calculation
    Mod2A     "a%"  Calculation
    Mod2B     "b%"  Calculation
    BitAndA   "a&"  Calculation
    BitAndB   "b&"  Calculation
    BitOrA    "a|"  Calculation
    BitOrB    "b|"  Calculation
    BitXorA   "a#"  Calculation
    BitXorB   "b#"  Calculation
    EqA       "a="  Calculation
    EqB       "b="  Calculation
    NeqA      "a!"  Calculation
    NeqB      "b!"  Calculation
    NonzeroA  "a1"  Calculation
    NonzeroB  "b1"  Calculation
    IsZeroA   "a0"  Calculation
    IsZeroB   "b0"  Calculation

    WaitA         ".a"   Control
    WaitB         ".b"   Control
    MoveL         "!<"   Control
    MoveR         "!>"   Control
    MoveU         "!^"   Control
    MoveD         "!v"   Control
    CondMoveL     "?<"   Control
    CondMoveR     "?>"   Control
    CondMoveU     "?^"   Control
    CondMoveD     "?v"   Control
    CondHalt      "?@"   Control
    ReflectAll    "!#"   Control
    ReflectX      "!|"   Control
    ReflectY      "!-"   Control
    ReflectFwd    "!/"   Control
    ReflectBwd    "!\\"  Control
    SetFlag       "(("   Control
    ClearFlag     "))"   Control
    FlagZeroA     "(a"   Control
    FlagNonzeroA  ")a"   Control
    FlagZeroB     "(b"   Control
    FlagNonzeroB  ")b"   Control
    FlagEq        "(="   Control
    FlagNeq       "(!"   Control
    FlagNot       ")("   Control
    FlagToA       "a("   Control
    FlagToB       "b("   Control

    CursorL        "#<"  Cursor
    CursorR        "#>"  Cursor
    CursorU        "#^"  Cursor
    CursorD        "#v"  Cursor
    CursorLTimesA  "a<"  Cursor
    CursorRTimesA  "a>"  Cursor
    CursorUTimesA  "a^"  Cursor
    CursorDTimesA  "av"  Cursor
    CursorLTimesB  "b<"  Cursor
    CursorRTimesB  "b>"  Cursor
    CursorUTimesB  "b^"  Cursor
    CursorDTimesB  "bv"  Cursor
    CursorHome     "#0"  Cursor

    RadiusA      "ra"  Selection
    RadiusB      "rb"  Selection
    RadiusReset  "r0"  Selection
    RadiusToA    "ar"  Selection
    RadiusToB    "br"  Selection
    IncRadius    "r+"  Selection
    DecRadius    "r-"  Selection
    CursorA      "ma"  Selection
    CursorB      "mb"  Selection
    CursorToA    "am"  Selection
    CursorToB    "bm"  Selection
    Copy         "cm"  Selection
    Paste        "mc"  Selection
}

impl Instruction {
    pub fn from_byte(b: u8) -> Self {
        INSTRUCTIONS.get(b as usize).copied().unwrap_or(Self::Nop)
    }
    pub fn from_symbol(symbol: &str) -> Option<Self> {
        INSTRUCTION_SYMBOLS.iter().position(|&s| s == symbol)
            .map(|b| Self::from_byte(b as u8))
    }
}