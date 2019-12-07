use termion::color;

#[derive(Clone, Copy)]
pub enum Category {
    Special,
    Calculation,
    Control,
    Cursor,
    Selection,
    Memory,
}

impl Category {
    pub fn fg_color_sequence(self) -> String {
        match self {
            Self::Special     => format!("{}", color::Fg(color::AnsiValue::grayscale(4))),
            Self::Calculation => format!("{}", color::Fg(color::LightGreen)),
            Self::Control     => format!("{}", color::Fg(color::LightMagenta)),
            Self::Cursor      => format!("{}", color::Fg(color::LightCyan)),
            Self::Selection   => format!("{}", color::Fg(color::LightRed)),
            Self::Memory      => format!("{}", color::Fg(color::LightBlue)),
        }
    }
}

macro_rules! gen_variant {
    (
        $enum_name:ident
        (
            $array_name:ident,
            $symbol_array_name:ident
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

gen_variant! { Instruction (INSTRUCTIONS, INSTRUCTION_SYMBOLS)
    Halt        "@@"  Special
    Nop         ".."  Special
    FlagFork    "-="  Special
    CursorFork  "m="  Special

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
    RadiusReset  "r1"  Selection
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
    Swap         "c:"  Selection

    Pointer0        "]0"  Memory
    PointerA        "]a"  Memory
    PointerB        "]b"  Memory
    PointerToA      "a]"  Memory
    PointerToB      "b]"  Memory
    PointerL        "]<"  Memory
    PointerR        "]>"  Memory
    PointerLTimesA  "}A"  Memory
    PointerRTimesA  "}a"  Memory
    PointerLTimesB  "}B"  Memory
    PointerRTimesB  "}b"  Memory
    Pointee0        "[0"  Memory
    PointeeA        "[a"  Memory
    PointeeB        "[b"  Memory
    PointeeToA      "a["  Memory
    PointeeToB      "b["  Memory
    IncPointee      "[+"  Memory
    DecPointee      "[-"  Memory
    IncPointeeA     "{a"  Memory
    DecPointeeA     "{A"  Memory
    IncPointeeB     "{b"  Memory
    DecPointeeB     "{B"  Memory
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