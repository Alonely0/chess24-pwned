use std::collections::HashMap;

use image::Rgba;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use crate::board::*;

#[derive(Debug, Deserialize)]
pub struct DataFile {
    #[serde(skip)]
    #[serde(rename(deserialize = "metadata"))]
    _metadata: (),
    #[serde(deserialize_with = "DataFile::de_cuepoints")]
    pub cuepoints: Box<[Instruction]>,
    #[serde(skip)]
    #[serde(rename(deserialize = "exerciseGroup"))]
    _exercise_groups: (),
    pub games: Box<[Game]>,
}

#[derive(Debug)]
pub struct Game {
    pub init: Fen,
    pub moves: HashMap<usize, Move>,
}

#[derive(Debug, Clone)]
pub struct Move {
    pub prev_m: usize,
    pub data: MoveData,
}

#[derive(Debug, Clone)]
pub enum MoveData {
    Fen(Fen),
    Coord(([[u32; 2]; 2], Option<usize>)),
}

#[derive(Debug)]
pub struct Instruction(pub (f64, InstructionData));

#[derive(Debug, Deserialize, Clone, Copy)]
#[repr(u32)]
pub enum Color {
    Yellow = 0xDBDB00FF,
    Green = 0x27DB33FF,
    Blue = 0x3327DBFF,
    Red = 0xDB3328FF,
}

#[derive(Debug, Clone)]
pub enum InstructionData {
    HighlightSquare {
        color: Color,
        coord: [u32; 2],
        game_index: usize,
    },
    DrawArrow {
        color: Color,
        coord: [[u32; 2]; 2],
        game_index: usize,
    },
    Unarrow {
        coord: [[u32; 2]; 2],
        game_index: usize,
    },
    UnarrowAll {
        game_index: usize,
    },
    ClearAllHighlights {
        game_index: usize,
    },
    GotoId {
        id: usize,
        game_index: usize,
    },
    Unmark {
        coord: [u32; 2],
        game_index: usize,
    },
    UnmarkAll {
        game_index: usize,
    },
    SelectGame {
        initial_move_id: Option<usize>,
        game_index: usize,
    },
    Move {
        id: usize,
        mov: usize,
        fen: Fen,
        game_index: usize,
    },
    Nop,
}

impl Color {
    fn from_str(str: &str) -> Self {
        match str {
            "yellow" => Self::Yellow,
            "green" => Self::Green,
            "blue" => Self::Blue,
            "red" => Self::Red,
            _ => panic!("{str}"),
        }
    }
}

impl From<Color> for Rgba<u8> {
    fn from(value: Color) -> Self { Self((value as u32).to_be_bytes()) }
}

impl DataFile {
    fn de_cuepoints<'de, D: Deserializer<'de>>(d: D) -> Result<Box<[Instruction]>, D::Error> {
        Box::<[Instruction]>::deserialize(d)
    }
}

macro_rules! get {
    ($d:ident, $n:expr, $t:ident) => {
        get!(@err $d.get($n).and_then(Value::$t), $n)
    };
    ($d:ident, coords) => {
        [get!($d, "x", as_u64) as u32, get!($d, "y", as_u64) as u32].map(|c| c + 1)
    };
    ($d:ident, line_as_can) => {
        line_as_can2coord(get!($d, "lineAsCan", as_str)).0
    };
    ($d:ident, game_index) => {
        get!($d, "gameIndex", as_u64) as usize
    };
    ($d:ident, color) => {
        Color::from_str(get!($d, "color", as_str))
    };
    ($d:ident, video_start_fen) => {
        Fen::new(get!(@err $d.get("video_start_fen").and_then(Value::as_str), "video_start_fen").to_owned())
    };
    (@err $x:expr, $n: expr) =>{
    $x.ok_or_else(|| serde::de::Error::custom(format!("Error deserializing {}", $n)))?
}}

impl<'de> Deserialize<'de> for Game {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Map::<String, Value>::deserialize(deserializer)?;
        Ok(Self {
            init: get!(raw, video_start_fen),
            moves: {
                let raw = get!(@err raw.get("moves"), "moves").as_array();
                let raw = get!(@err raw, "moves");

                raw.iter()
                    .map(|x| {
                        Ok((
                            get!(x, "id", as_u64) as _,
                            Move {
                                prev_m: get!(x, "pm", as_i64) as usize,
                                data: {
                                    if let Some(fen) = x.get("fen").and_then(Value::as_str) {
                                        MoveData::Fen(Fen::new(fen.to_owned()))
                                    } else {
                                        MoveData::Coord(line_as_can2coord(get!(x, "m", as_str)))
                                    }
                                },
                            },
                        ))
                    })
                    .collect::<Result<_, _>>()?
            },
        })
    }
}

impl<'de> Deserialize<'de> for Instruction {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        InstructionData::des(d).map(Self)
    }
}
impl InstructionData {
    fn des<'de, D: Deserializer<'de>>(d: D) -> Result<(f64, Self), D::Error> {
        #[derive(Deserialize)]
        struct Cuepoint {
            name: String,
            time: f64,
            data: Value,
        }
        let raw = Cuepoint::deserialize(d)?;
        let data = raw.data;
        Ok((
            raw.time,
            match raw.name.as_str() {
                "gotoId" => Self::GotoId {
                    id: get!(data, "id", as_u64) as _,
                    game_index: get!(data, game_index),
                },
                "selectGame" => Self::SelectGame {
                    initial_move_id: data
                        .get("initialMoveId")
                        .and_then(Value::as_u64)
                        .map(|x| x as usize),
                    game_index: get!(data, game_index),
                },
                "highlightSquare" => Self::HighlightSquare {
                    color: get!(data, color),
                    coord: get!(data, coords),
                    game_index: get!(data, game_index),
                },
                "drawArrow" => Self::DrawArrow {
                    color: get!(data, color),
                    coord: get!(data, line_as_can),
                    game_index: get!(data, game_index),
                },
                "unmark" => {
                    Self::Unmark { coord: get!(data, coords), game_index: get!(data, game_index) }
                }
                "unmarkAll" => Self::UnmarkAll { game_index: get!(data, game_index) },
                "clearAllHighlights" => {
                    Self::ClearAllHighlights { game_index: get!(data, game_index) }
                }
                "move" => Self::Move {
                    id: get!(data, "id", as_u64) as _,
                    mov: get!(data, "move", as_u64) as _,
                    fen: Fen::new(get!(data, "fen", as_str).to_owned()),
                    game_index: get!(data, game_index),
                },
                "unarrow" => Self::Unarrow {
                    coord: get!(data, line_as_can),
                    game_index: get!(data, game_index),
                },
                "unarrowAll" => Self::UnarrowAll { game_index: get!(data, game_index) },
                "triggerExerciseGroup" => Self::Nop, // uninmplemented
                _ => unimplemented!("{:?}", raw.name),
            },
        ))
    }
}
fn line_as_can2coord(str: &str) -> ([[u32; 2]; 2], Option<usize>) {
    let parse = |s: &str| {
        let mut chars = s.chars();
        let [l, n] = [chars.next(), chars.next()].map(Option::unwrap);
        [l as u8 - b'a' + 1, n as u8 - b'0'].map(u32::from)
    };
    (
        [&str[0..2], &str[2..4]].map(parse),
        str.chars().nth(4).and_then(Piece::uncolored),
    )
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Fen(String);

#[derive(Debug, Copy, Clone)]
pub struct RefFen<'a> {
    row: u32,
    column: u32,
    fen: &'a str,
    skip: u32,
    i: usize,
}

impl Fen {
    fn new(fen: String) -> Self { Self(fen) }

    pub fn iter(&self) -> RefFen<'_> { RefFen { row: 8, column: 1, fen: &self.0, skip: 0, i: 0 } }
}

impl<'a> Iterator for RefFen<'a> {
    type Item = ([u32; 2], Option<&'static Piece>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.skip > 0 {
                let ret = Some(([self.column, self.row], None));
                self.skip -= 1;
                self.column += 1;
                break ret;
            } else if self.row != 0 {
                let c = self.fen.chars().nth(self.i).unwrap();
                if c.is_ascii_digit() {
                    self.skip = u32::from(c as u8 - b'0');
                } else {
                    let ret = Some((
                        [self.column, self.row],
                        Some(match Piece::from_char(c) {
                            Some(p) => p,
                            None => match c {
                                '\\' => {
                                    self.i += 1;
                                    continue;
                                }
                                '/' | ' ' => {
                                    self.column = 1;
                                    self.row -= 1;
                                    self.i += 1;
                                    continue;
                                }
                                _ => unreachable!("{c}"),
                            },
                        }),
                    ));
                    self.column += 1;
                    self.i += 1;
                    break ret;
                }
            } else {
                break None;
            }
            self.i += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(64)) }
}

impl Piece {
    pub fn uncolored(c: char) -> Option<usize> {
        Some(match c.to_ascii_uppercase() {
            'P' => 0,
            'N' => 1,
            'B' => 2,
            'R' => 3,
            'Q' => 4,
            'K' => 5,
            _ => return None,
        })
    }

    pub fn from_char(c: char) -> Option<&'static Self> {
        Some(Pieces[c.is_ascii_lowercase() as u8 as usize][Self::uncolored(c)?])
    }

    pub fn from_uncolored(original: Option<&'static Self>, n: usize) -> Option<&'static Self> {
        original.map(|x| x.1 as u8 as usize).map(|c| Pieces[c][n])
    }
}
