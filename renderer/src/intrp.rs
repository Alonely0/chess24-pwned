#![allow(unused_variables)]
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{
    board::{Chessboard, Piece},
    comp,
    instr::{Color, DataFile, Game, Instruction, InstructionData, MoveData},
    intrp::seal::TM,
    str,
};

pub struct Interpreter {
    data: DataFile,
    timeline: TM,
    concat: String,
    last_visited: HashMap<usize, usize>,
}

impl Interpreter {
    pub fn new(data: DataFile) -> Self {
        Self {
            data,
            timeline: TM::new(),
            concat: String::with_capacity(16 * 1024),
            last_visited: HashMap::new(),
        }
    }

    pub fn render_frames(mut self, out: impl AsRef<Path>) -> (PathBuf, Option<f64>) {
        let mut out = out.as_ref().to_owned().canonicalize().unwrap();
        let mut iter = self
            .data
            .cuepoints
            .into_vec()
            .into_iter()
            .enumerate()
            .peekable();
        let t = 'render: loop {
            if let Some((i, Instruction((t, instr)))) = iter.next() {
                let (next_t, mut end) = match iter.peek() {
                    Some((_, Instruction((t, _)))) => (*t, false),
                    None => (t + 1.0, true),
                };
                let d_t = next_t - t;
                match instr {
                    // WONTFIX: highlights integrated in gotoid
                    InstructionData::GotoId { id, game_index } => {
                        Self::goto_id([id, game_index], &mut self.timeline, &mut self.data.games);
                    }
                    InstructionData::Move { id, mov, fen, game_index } => {
                        let mut board = self.timeline.get().clone();
                        board.clear_markers();

                        let mut df = Vec::with_capacity(2);
                        for (pos @ [r, c], p) in fen.iter() {
                            if comp!(neq, board.state[r as usize - 1][c as usize - 1], p) {
                                df.push((pos, p))
                            }
                            board.draw_piece(pos, p)
                        }
                        if df.len() > 3 || df.len() < 2 {
                        } else if df[0].1.is_none() && df[1].1.is_some() {
                            board.arrow([df[0].0, df[1].0], Color::Blue);
                        } else if df[1].1.is_none() && df[0].1.is_some() {
                            board.arrow([df[1].0, df[0].0], Color::Blue);
                        }
                        self.timeline.insert([id, game_index], board);
                    }
                    InstructionData::DrawArrow { color, coord, game_index } => {
                        self.timeline.get().arrow(coord, color)
                    }
                    InstructionData::HighlightSquare { color, coord, game_index } => {
                        self.timeline.get().highlt(coord, color)
                    }
                    InstructionData::Unmark { coord, game_index } => {
                        self.timeline.get().unhighlt(&coord)
                    }
                    InstructionData::Unarrow { coord, game_index } => {
                        self.timeline.get().unarrow(&coord)
                    }
                    InstructionData::UnmarkAll { game_index } => {
                        self.timeline.get().clear_markers()
                    }
                    InstructionData::ClearAllHighlights { game_index } => {
                        self.timeline.get().clear_highlt()
                    }
                    InstructionData::UnarrowAll { game_index } => {
                        self.timeline.get().clear_arrows()
                    }
                    InstructionData::SelectGame { initial_move_id, game_index } => {
                        // handle corrupted datafiles :'(
                        let Some(game) = self.data.games.get_mut(game_index) else {
                            break 'render Some(t);
                        };

                        if let Some([last_m, last_g]) = self.timeline.get_key().copied() {
                            self.last_visited.insert(last_g, last_m);
                        }

                        if let Some(id) = initial_move_id {
                            let mut board = Chessboard::new();
                            for (c, p) in &mut game.init.iter() {
                                board.draw_piece(c, p);
                            }
                            if let Some(&mut MoveData::Coord((c, p))) =
                                game.moves.get_mut(&id).as_mut().map(|x| &mut x.data)
                            {
                                board.arrow(c, Color::Blue);
                                if let Some(p) = p {
                                    board.draw_piece(
                                        c[1],
                                        Some(
                                            Piece::from_uncolored(
                                                board.state[c[1][0] as usize - 1]
                                                    [c[1][1] as usize - 1],
                                                p,
                                            )
                                            .unwrap(),
                                        ),
                                    );
                                }
                            }
                            self.timeline.insert([id, game_index], board);
                        } else {
                            self.timeline.index_of(&[
                                *self.last_visited.get(&game_index).unwrap(),
                                game_index,
                            ]);
                        }
                    }
                    InstructionData::Nop => {}
                };
                out.push(&format!("{i}.png"));
                self.timeline.get().save(&out);
                // duplicate last, ffmpeg bug
                'save: loop {
                    self.concat
                        .push_str(&format!("file '{}'\nduration {d_t}\n", str(&out)));
                    end = !end;
                    if end {
                        break 'save;
                    }
                }
                out.pop();
            } else {
                break 'render None;
            }
        };
        out.push("concat.txt");
        BufWriter::new(File::create(&out).unwrap())
            .write_all(self.concat.as_bytes())
            .unwrap();
        (out, t)
    }

    #[inline]
    fn goto_id(
        mov @ [id, game_index]: [usize; 2],
        timeline: &mut TM,
        games: &mut Box<[Game]>,
    ) -> usize {
        if timeline.index_of(&mov).is_none() {
            match games[game_index].moves.get(&id) {
                Some(m) if id != m.prev_m => {
                    let i = Self::goto_id([m.prev_m, game_index], timeline, games);
                    unsafe { timeline.set_cursor(i) };
                }
                _ => {}
            };
            let mut board = timeline.get().clone();
            Self::mov(&mut board, games, mov);
            timeline.insert(mov, board);
        }
        timeline.index()
    }

    fn mov(board: &mut Chessboard, games: &mut Box<[Game]>, [id, game_index]: [usize; 2]) {
        board.clear_markers();
        let co = |board: &mut Chessboard, c| {
            board.move_piece(c);
            board.arrow(c, Color::Blue);
        };
        if let Some(mov) = games[game_index].moves.get_mut(&id) {
            match &mut mov.data {
                MoveData::Coord((c, None)) => co(board, *c),
                MoveData::Coord((c, Some(p))) => {
                    co(board, *c);
                    board.draw_piece(
                        c[1],
                        Some(
                            Piece::from_uncolored(
                                board.state[c[1][0] as usize - 1][c[1][1] as usize - 1],
                                *p,
                            )
                            .unwrap(),
                        ),
                    );
                }
                MoveData::Fen(fen) => {
                    for (c, p) in fen.iter() {
                        board.draw_piece(c, p);
                    }
                }
            }
        }
    }
}

impl Chessboard {
    #[inline]
    fn clear_markers(&mut self) {
        self.clear_arrows();
        self.clear_highlt();
    }
}

mod seal {
    use std::hash::Hash;

    use indexmap::IndexMap;

    pub(super) type TM = CursoredAppendOnlyIM<[usize; 2], super::Chessboard>;

    pub(super) struct CursoredAppendOnlyIM<K, V> {
        inner: IndexMap<K, V>,
        index: usize,
    }

    impl<K: Hash + Eq + Default, V: Default> CursoredAppendOnlyIM<K, V> {
        #[inline]
        pub(super) fn new() -> Self { Self { inner: IndexMap::new(), index: 0 } }

        #[inline]
        pub(super) fn get(&mut self) -> &mut V {
            if self.inner.is_empty() {
                unreachable!("{}", self.index)
            };
            unsafe { self.inner.get_index_mut(self.index).unwrap_unchecked().1 }
        }

        #[inline]
        pub(super) fn get_key(&mut self) -> Option<&K> {
            if self.inner.is_empty() {
                return None;
            }
            unsafe { Some(self.inner.get_index_mut(self.index).unwrap_unchecked().0) }
        }

        #[inline]
        pub(super) fn insert(&mut self, k: K, v: V) { self.index = self.inner.insert_full(k, v).0; }

        #[inline]
        pub(super) fn index_of(&mut self, k: &K) -> Option<()> {
            self.index = self.inner.get_index_of(k)?;
            Some(())
        }

        #[inline]
        pub(super) unsafe fn set_cursor(&mut self, i: usize) { self.index = i; }

        #[inline]
        pub(super) fn index(&self) -> usize { self.index }
    }
}
