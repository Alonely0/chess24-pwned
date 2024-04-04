#![allow(non_upper_case_globals)]
use core::slice;
use std::{collections::HashMap, f64::consts::PI, mem::take, path::Path};

use image::{ImageBuffer, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use palette::{
    blend::{Blend, Compose, PreAlpha},
    LinSrgba,
};

macro_rules! incl {
    ($(($t:tt $x:ident: $y:expr)),+) => {
        $(incl!(@dispatch $t $x $y);)+
    };
    (@dispatch White $x:ident $y:expr) => {
        incl!(@decl White $x $y, false);
    };
    (@dispatch Black $x:ident $y:expr) => {
        incl!(@decl Black $x $y, true);
    };
    (@decl $x:ident $y:ident $z:expr, $b:expr) => {
        paste::paste!{
            pub static [<$x$y>]: Piece = Piece(
                Lazy::new(|| image::open($z).unwrap().to_rgba8()), $b
            );
        }
    }
}

macro_rules! pieces {
    ([$($c:ident),+] @ $p:tt) => {
        pub static Pieces: [[&Piece; 6]; 2] = [$(pieces!(@recurse $c $p)),+];
    };
    (@recurse $c:ident [$($p:ident),+]) => {
        [$(&paste::paste!([<$c$p>])),+]
    }
}

incl!(
    (White King: "./assets/klt.png"),
    (Black King: "./assets/kdt.png"),
    (White Queen: "./assets/qlt.png"),
    (Black Queen: "./assets/qdt.png"),
    (White Rook: "./assets/rlt.png"),
    (Black Rook: "./assets/rdt.png"),
    (White Bishop: "./assets/blt.png"),
    (Black Bishop: "./assets/bdt.png"),
    (White Knight: "./assets/nlt.png"),
    (Black Knight: "./assets/ndt.png"),
    (White Pawn: "./assets/plt.png"),
    (Black Pawn: "./assets/pdt.png")
);

pieces!([White, Black] @ [Pawn, Knight, Bishop, Rook, Queen, King]);

#[macro_export]
macro_rules! comp {
    (eq, $a:expr, $b:expr) => {
        std::ptr::eq::<()>(unsafe { std::mem::transmute($a) }, unsafe {
            std::mem::transmute($b)
        })
    };
    (neq, $a:expr, $b:expr) => {
        !comp!(eq, $a, $b)
    };
}

macro_rules! th {
    (x: $e:expr) => {
        (
            $e.into_iter(),
            [0].into_iter().cycle().take($e.into_iter().count()),
        )
    };
    (y: $e:expr) => {
        (
            [0].into_iter().cycle().take($e.into_iter().count()),
            $e.into_iter(),
        )
    };
    ((x: $e0:expr,y: $e1:expr)) => {
        ($e1.into_iter(), $e1.into_iter())
    };
}

macro_rules! br {
    ($r:expr, $s:expr, $v:expr, [$($x:expr),+]) => {
{        let a = core::iter::empty();
        $(
            let a = a.chain($r($s + $v * $x));
        )+
    a}
    };
}

#[derive(Debug)]
pub struct Piece(Lazy<RgbaImage>, pub bool);

pub struct Chessboard {
    buf: Box<[Rgba<u8>]>,
    pub state: [[Option<&'static Piece>; 8]; 8],
    arrows: HashMap<[[u32; 2]; 2], Rgba<u8>>,
    redraw_arrows: bool,
}

impl Chessboard {
    pub const ARROWS_LAYER: usize = 1;
    pub const BO_SIZE: u32 = 536;
    pub const HIGHLT_LAYER: usize = 2;
    pub const IMGS: [usize; 3] = {
        const fn a(n: usize) -> usize { Chessboard::LEN * n }
        [a(0), a(1), a(2)]
    };
    pub const LEN: usize = Self::BO_SIZE.pow(2) as usize;
    pub const SQ_N_E: u32 = 8;
    pub const SQ_SIZE: u32 = 67;

    pub fn new() -> Self {
        let buf = vec![Rgba([0u8; 4]); Self::LEN * 3].into_boxed_slice();
        let mut board = Self {
            state: [[None; 8]; 8],
            arrows: HashMap::with_capacity(6),
            buf,
            redraw_arrows: true,
        };
        board.iter_pixels_mut(0).for_each(|(i, p)| {
            let colors_buf = [
                Rgba([0x7D, 0x3E, 0x2F, 0xFF]),
                Rgba([0xA6, 0x80, 0x67, 0xFF]),
                Rgba([0x7D, 0x3E, 0x2F, 0xFF]),
            ];
            let pos = usize::from(((i / Self::BO_SIZE) / Self::SQ_SIZE) % 2 == 0);
            *p = colors_buf[pos..=(pos + 1)]
                [usize::from(((i % Self::BO_SIZE) / Self::SQ_SIZE) % 2 != 0)];
        });
        board
    }

    pub fn img(&self, n: usize) -> &[Rgba<u8>] {
        unsafe { slice::from_raw_parts(self.buf.as_ptr().add(Self::IMGS[n]), Self::LEN) }
    }

    pub fn img_mut(&mut self, n: usize) -> &mut [Rgba<u8>] {
        unsafe { slice::from_raw_parts_mut(self.buf.as_mut_ptr().add(Self::IMGS[n]), Self::LEN) }
    }

    pub fn arrow(&mut self, coord: [[u32; 2]; 2], color: impl Into<Rgba<u8>>) {
        self.arrows.insert(coord, color.into());
        self.redraw_arrows = true;
    }

    pub fn unarrow(&mut self, coord: &[[u32; 2]; 2]) {
        self.arrows.remove(coord);
        self.redraw_arrows = true;
    }

    pub fn clear_arrows(&mut self) {
        self.arrows.clear();
        self.redraw_arrows = true;
    }

    pub fn highlt(&mut self, coord: [u32; 2], color: impl Into<Rgba<u8>>) {
        let color = color.into();
        let layer = self.img_mut(Self::HIGHLT_LAYER);
        let bo_size = Self::BO_SIZE as usize;
        let sq_size = Self::SQ_SIZE as usize;
        let start = (coord[0] as usize - 1) * sq_size + (8 - coord[1] as usize) * bo_size * sq_size;
        let r = |x: usize| x..x + sq_size;

        br!(r, start, bo_size, [0, 1, 2, 3, 4])
            .chain(br!(r, start, bo_size, [62, 63, 64, 65, 66]))
            .chain((1..sq_size - 2).flat_map(|i| {
                (start + bo_size * i..start + bo_size * i + 5)
                    .chain(start + bo_size * i + sq_size - 5..start + bo_size * i + sq_size)
            }))
            .for_each(|x| layer[x] = color);
    }

    pub fn unhighlt(&mut self, coord: &[u32; 2]) { self.highlt(*coord, Rgba([0; 4])) }

    pub fn clear_highlt(&mut self) { self.img_mut(Self::HIGHLT_LAYER).fill(Rgba([0; 4])) }

    #[cfg(test)]
    pub fn draw_initial(&mut self) {
        for j in 1..=Self::SQ_N_E {
            self.draw_piece([j, 2], Some(&WhitePawn));
        }
        self.draw_piece([1, 1], Some(&WhiteRook));
        self.draw_piece([Self::SQ_N_E, 1], Some(&WhiteRook));
        self.draw_piece([2, 1], Some(&WhiteKnight));
        self.draw_piece([Self::SQ_N_E - 1, 1], Some(&WhiteKnight));
        self.draw_piece([3, 1], Some(&WhiteBishop));
        self.draw_piece([Self::SQ_N_E - 2, 1], Some(&WhiteBishop));
        self.draw_piece([4, 1], Some(&WhiteQueen));
        self.draw_piece([5, 1], Some(&WhiteKing));

        for j in 1..=Self::SQ_N_E {
            self.draw_piece([j, Self::SQ_N_E - 1], Some(&BlackPawn));
        }
        self.draw_piece([1, Self::SQ_N_E], Some(&BlackRook));
        self.draw_piece([Self::SQ_N_E, Self::SQ_N_E], Some(&BlackRook));
        self.draw_piece([2, Self::SQ_N_E], Some(&BlackKnight));
        self.draw_piece([Self::SQ_N_E - 1, Self::SQ_N_E], Some(&BlackKnight));
        self.draw_piece([3, Self::SQ_N_E], Some(&BlackBishop));
        self.draw_piece([Self::SQ_N_E - 2, Self::SQ_N_E], Some(&BlackBishop));
        self.draw_piece([4, Self::SQ_N_E], Some(&BlackQueen));
        self.draw_piece([5, Self::SQ_N_E], Some(&BlackKing));
    }

    fn get_square(
        &mut self,
        [x, y]: [u32; 2],
        z: usize,
    ) -> impl Iterator<Item = (u32, &mut Rgba<u8>)> {
        self.iter_pixels_mut(z).filter_map(move |(i, p)| {
            if (i % Self::BO_SIZE) < Self::SQ_SIZE * x
                && (i % Self::BO_SIZE) >= Self::SQ_SIZE * (x - 1)
                && i / Self::BO_SIZE >= (Self::SQ_N_E - y) * Self::SQ_SIZE
                && i / Self::BO_SIZE < (Self::SQ_N_E - (y - 1)) * Self::SQ_SIZE
            {
                Some((i, p))
            } else {
                None
            }
        })
    }

    pub fn draw_line(
        &mut self,
        img: usize,
        color: impl Into<Rgba<u8>>,
        (th_0, th_1): (impl Iterator<Item = i32>, impl Iterator<Item = i32>),
        [[a_x, a_y], [b_x, b_y]]: [[i32; 2]; 2],
    ) {
        let color = color.into();
        th_0.into_iter()
            .zip(th_1)
            .flat_map(|(i_x, i_y)| {
                Self::bresenham([[a_x + i_x, a_y + i_y], [b_x + i_x, b_y + i_y]])
            })
            .for_each(|[x, y]| {
                self.img_mut(img)[(x.unsigned_abs() + y.unsigned_abs() * Self::BO_SIZE) as usize] =
                    color;
            });
    }

    pub fn bresenham(
        p @ [[a_x, a_y], [b_x, b_y]]: [[i32; 2]; 2],
    ) -> impl Iterator<Item = [i32; 2]> {
        struct Bresenham {
            p: [[i32; 2]; 2],
            d: [i32; 2],
            e: i32,
            s: [i32; 2],
        }

        impl Iterator for Bresenham {
            type Item = [i32; 2];

            fn next(&mut self) -> Option<Self::Item> {
                let e2 = self.e;

                if e2 > -self.d[0] {
                    self.e -= self.d[1];
                    self.p[0][0] += self.s[0];
                }
                if e2 < self.d[1] {
                    self.e += self.d[0];
                    self.p[0][1] += self.s[1];
                }
                if self.p[0][0] == self.p[1][0] && self.p[0][1] == self.p[1][1] {
                    return None;
                }

                Some(self.p[0])
            }
        }
        let d @ [d_x, d_y] = [(b_x - a_x).abs(), (b_y - a_y).abs()];

        Bresenham {
            p,
            d,
            e: if d_x > d_y { d_x } else { -d_y } / 2,
            s: [if a_x < b_x { 1 } else { -1 }, if a_y < b_y { 1 } else { -1 }],
        }
    }

    #[inline]
    pub fn draw_piece(&mut self, [r, c]: [u32; 2], piece: Option<&'static Piece>) {
        self.state[r as usize - 1][c as usize - 1] = piece;
    }

    pub fn move_piece(&mut self, [[x_r, x_c], [y_r, y_c]]: [[u32; 2]; 2]) {
        let [x_r, x_c, y_r, y_c] = [x_r, x_c, y_r, y_c].map(|x| x as usize);
        if x_r == 5
            && (x_c == 1 || x_c == 8)
            && x_c == y_c
            && (x_r as isize - y_r as isize).abs() > 1
            && (comp!(eq, self.state[x_r - 1][x_c - 1], &WhiteKing)
                || comp!(eq, self.state[x_r - 1][x_c - 1], &BlackKing))
        {
            if y_r == 7 {
                self.state[y_r - 2][y_c - 1] = self.state[y_r][y_c - 1].take();
            } else if y_r == 3 {
                self.state[y_r][y_c - 1] = self.state[0][y_c - 1].take();
            } else {
                unreachable!("{:?}", [[x_r, x_c,], [y_r, y_c]])
            };
        } else if self.state[y_r - 1][y_c - 1].is_none()
            && ((comp!(eq, self.state[x_r - 1][x_c - 1], &WhitePawn)
                && comp!(eq, self.state[y_r - 1][x_c - 1], &BlackPawn))
                || (comp!(eq, self.state[x_r - 1][x_c - 1], &WhitePawn)
                    && comp!(eq, self.state[y_r - 1][x_c - 1], &BlackPawn)))
        {
            self.state[y_r - 1][x_c - 1] = None;
        }
        self.state[y_r - 1][y_c - 1] = self.state[x_r - 1][x_c - 1].take();
    }

    fn iter_pixels_mut(&mut self, i: usize) -> impl Iterator<Item = (u32, &mut Rgba<u8>)> {
        self.img_mut(i)
            .iter_mut()
            .enumerate()
            .map(|(i, p)| (i as u32, p))
    }

    fn prerender_arrows(&mut self) {
        if !self.redraw_arrows {
            return;
        }
        self.img_mut(Self::ARROWS_LAYER).fill(Rgba([0; 4]));
        let arrows = take(&mut self.arrows);
        for (coord, color) in &arrows {
            let color = *color;
            let mut p = |c| {
                self.get_square(c, Chessboard::ARROWS_LAYER)
                    .nth((Self::SQ_SIZE * (Self::SQ_SIZE - 1)) as usize)
                    .unwrap()
                    .0
                    - Self::BO_SIZE * (Self::SQ_SIZE / 2) * 2
            };
            let separate_coord = |c: u32| {
                let c_x = |n| (n % Self::BO_SIZE) as i32;
                let c_y = |n| (n / Self::BO_SIZE) as i32;
                [c_x(c), c_y(c)]
            };
            let c @ [[x1, y1], [x2, y2]] =
                coord.map(|c| separate_coord(p(c)).map(|a| a + Self::SQ_SIZE as i32 / 2));
            let th = -1..=1;
            if y1 == y2 {
                self.draw_line(Self::ARROWS_LAYER, color, th!(y: -3..=3), c);
            } else {
                self.draw_line(Self::ARROWS_LAYER, color, th!(x: -3..=3), c);
            }
            let a_offset = if x1 == x2 {
                PI * f64::from(u8::from(y1 > y2))
            } else {
                let slope =
                    |[[x1, y1], [x2, y2]]: [[i32; 2]; 2]| (y2 - y1) as f64 / (x2 - x1) as f64;
                let [s1, s2] = [slope(c), 0.0];
                let is_knight_movement_v = || {
                    (coord[0][0] as i32 - coord[1][0] as i32).abs() == 1
                        && (coord[0][1] as i32 - coord[1][1] as i32).abs() == 2
                };
                let is_knight_movement_h = || {
                    (coord[0][0] as i32 - coord[1][0] as i32).abs() == 2
                        && (coord[0][1] as i32 - coord[1][1] as i32).abs() == 1
                };
                (s2 - s1 / (1.0 + s1 * s2)).abs().atan()
                    + if x1 > x2 && y1 >= y2 {
                        PI / 2.0
                    } else if x1 > x2 && y1 < y2 {
                        f64::from(u8::from(is_knight_movement_h())) * PI / 5.0
                            + f64::from(u8::from(is_knight_movement_v())) * ((PI / 4.0) * PI + PI)
                    } else if x1 < x2 && y1 == y2 {
                        PI / -2.0
                    } else if x1 < x2 && y1 > y2 {
                        if is_knight_movement_v() {
                            PI.powi(2) / 4.0
                        } else if is_knight_movement_h() {
                            -PI.powi(2) / 4.0
                        } else {
                            PI
                        }
                    } else if x1 < x2 && y1 < y2 {
                        PI / -2.0
                    } else {
                        unreachable!("{c:?}\n")
                    }
            };
            let s = [PI / 4.0 + a_offset, PI / -4.0 + a_offset]
                .map(|a| {
                    let (sin_a, cos_a) = a.sin_cos();
                    let [o_x, o_y] = c[1].map(f64::from);
                    let [x, y] = [0.0, (-67.0f64 / 2.0).ceil()];
                    [
                        c[1],
                        [
                            (o_x + (x * cos_a - y * sin_a)) as i32,
                            (o_y + (y * cos_a + x * sin_a)) as i32,
                        ],
                    ]
                })
                .map(|c| {
                    self.draw_line(Self::ARROWS_LAYER, color, th!(y: th.clone()), c);
                    c[1]
                });
            self.draw_line(Self::ARROWS_LAYER, color, th!(y: th.clone()), s);
            for p in Self::bresenham(s) {
                self.draw_line(Self::ARROWS_LAYER, color, th!(y: th.clone()), [c[1], p]);
            }
        }
        self.arrows = arrows;
        self.redraw_arrows = false;
    }

    fn render_pieces(&self) -> impl Iterator<Item = &'static Rgba<u8>> + '_ {
        struct StatePixelGetter<'a>(&'a Chessboard, u32);
        impl<'a> Iterator for StatePixelGetter<'a> {
            type Item = &'static Rgba<u8>;

            fn next(&mut self) -> Option<Self::Item> {
                let c_n = self.1 / Chessboard::BO_SIZE;
                let d_n = self.1 / Chessboard::SQ_SIZE;
                let p_x = self.1 % Chessboard::SQ_SIZE;
                self.1 += 1;
                self.0
                    .state
                    .get((d_n % Chessboard::SQ_N_E) as usize)
                    .and_then(|s| {
                        s.get(7 - ((c_n / Chessboard::SQ_SIZE) as usize))
                            .map(|p| match p {
                                Some(i) => i.0.get_pixel(p_x, c_n % Chessboard::SQ_SIZE),
                                None => &Rgba([0x00; 4]),
                            })
                    })
            }
        }
        StatePixelGetter(self, 0)
    }

    pub fn render(&mut self) -> RgbaImage {
        let mut render = ImageBuffer::new(Self::BO_SIZE, Self::BO_SIZE);
        self.prerender_arrows();
        render
            .pixels_mut()
            .zip(self.img(0))
            .zip(self.img(Self::HIGHLT_LAYER))
            .zip(self.render_pieces())
            .zip(self.img(Self::ARROWS_LAYER))
            .for_each(|((((out, board), highlt), pieces), arrows)| {
                // arrows[3] &= !highlt[3];
                let [arrows, pieces, highlt, board] =
                    [arrows, pieces, highlt, board].map(|Rgba(c)| {
                        let [r, g, b, a] = c.map(|x| (x as f32) / 255.0);
                        PreAlpha::from(LinSrgba::new(r, g, b, a))
                    });
                let (r, g, b) = pieces
                    .over(highlt.over(board).overlay(arrows))
                    .into_components();
                *out = Rgba([r, g, b, 1.0].map(|x| (x * 255.0) as u8));
            });
        render
    }

    pub fn save(&mut self, path: impl AsRef<Path>) {
        let img = self.render();
        img.save(path).unwrap();
    }
}

impl Default for Chessboard {
    fn default() -> Self { Self::new() }
}

impl Clone for Chessboard {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            state: self.state,
            arrows: self.arrows.clone(),
            redraw_arrows: self.redraw_arrows,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.state = source.state;
        unsafe {
            std::ptr::copy_nonoverlapping(source.buf.as_ptr(), self.buf.as_mut_ptr(), Self::LEN * 3)
        }
        self.arrows.clone_from(&source.arrows);
        self.redraw_arrows = source.redraw_arrows;
    }
}

#[test]
#[cfg(test)]
pub fn test_board() {
    let mut board = Chessboard::new();
    board.draw_initial();
    board.move_piece([[3, 2], [3, 3]]);
    board.move_piece([[8, 1], [3, 3]]);
    board.highlt([3, 3], Rgba([0xFF, 0x00, 0x00, 0x99]));
    board.arrow([[8, 1], [3, 6]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[8, 2], [8, 1]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[2, 8], [3, 6]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [5, 3]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [1, 3]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [3, 5]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [3, 1]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [2, 4]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [4, 4]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [4, 2]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [2, 2]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [4, 5]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [2, 5]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [4, 1]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [2, 1]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [5, 4]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [1, 2]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [1, 4]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.arrow([[3, 3], [5, 2]], Rgba([0x00, 0xDD, 0x00, 0xFF]));
    board.save("./a.png");
}
