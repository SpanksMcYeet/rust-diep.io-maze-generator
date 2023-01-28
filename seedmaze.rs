/***

YOU CAN TEST IT HERE:
https://play.rust-lang.org/?version=nightly&mode=release&edition=2021&gist=1ccb6b3713e9915d221920503f9a0b74

***/

use std::collections::HashSet;

use rand::Rng;

struct SeededGenerator {
    seed: u64,
}
impl SeededGenerator {
    fn new(mut seed: u64) -> Self {
        seed %= 2147483647;
        if seed == 0 {
            seed = 2147483647;
        }
        Self { seed }
    }
    fn next(&mut self) -> u64 {
        self.seed = self.seed * 16807 % 2147483647;
        self.seed
    }
    fn next_float(&mut self) -> f64 {
        (self.next() as f64 - 1.) / 2147483646.
    }
    fn next_int(&mut self, max: u32) -> u32 {
        (self.next_float() * max as f64) as u32
    }
}

fn cyrb53(string: &str) -> u64 {
    let mut h1 = 0xdeadbeef;
    let mut h2 = 0x41c6ce57;
    for i in string.chars() {
        let ch = i as u32;
        h1 = (h1 ^ ch).wrapping_mul(2654435761);
        h2 = (h2 ^ ch).wrapping_mul(1597334677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2246822507) ^ (h2 ^ (h2 >> 13)).wrapping_mul(3266489909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2246822507) ^ (h1 ^ (h1 >> 13)).wrapping_mul(3266489909);
    ((2097151 & h2) as u64) << 32 | h1 as u64
}

fn parse_seed(seed: &str) -> u64 {
    if seed.is_empty() {
        rand::thread_rng().gen_range(0..2147483646)
    } else {
        seed.parse::<u64>().unwrap_or_else(|_| cyrb53(seed))
    }
}

struct SeedRange {
    min: u32,
    max: u32,
}

struct Config {
    size: u32,
    maze_seed: u64,
    prng: SeededGenerator,
    seed_range: SeedRange,
    turn_chance: f64,
    termination_chance: f64,
    wall_wrapping: bool,
}

struct Seed {
    x: u32,
    y: u32,
}

struct Wall {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Cell {
    Blank,
    ConnectedBlank,
    Filled,
    Colored(u8),
}

struct SeedMaze {
    prng: SeededGenerator,
    seed_amount: u32,
    turn_chance: f64,
    termination_chance: f64,
    wall_wrapping: bool,
    seeds: Vec<Seed>,
    map: SquareMap,
}

struct SquareMap {
    size: u32,
    map: Vec<Cell>,
    maze_seed: u64,
    walls: Vec<Wall>,
}

impl SquareMap {
    fn new(size: u32, maze_seed: u64) -> Self {
        let map = vec![Cell::Blank; size as usize * size as usize];
        let walls = Vec::new();
        Self { size, map, maze_seed, walls }
    }
    fn get(&mut self, x: u32, y: u32) -> Cell {
        self.map[y as usize * self.size as usize + x as usize]
    }
    fn set(&mut self, x: u32, y: u32, value: Cell) {
        self.map[y as usize * self.size as usize + x as usize] = value;
    }
    fn replace(&mut self, value: Cell, with: Cell) {
        for cell in &mut self.map {
            if *cell == value {
                *cell = with;
            }
        }
    }
    fn combine(&mut self) {
        let mut current_color = 0;
        for x in 0..self.size {
            for y in 0..self.size {
                if self.get(x, y) != Cell::Filled {
                    continue;
                }
                let color = current_color;
                current_color += 1;
                let mut wall = Wall { x, y, width: 0, height: 1 };
                while self.get(x + wall.width, y) == Cell::Filled {
                    self.set(x + wall.width, y, Cell::Colored(color));
                    wall.width += 1;
                }
                'joe: loop {
                    for i in 0..wall.width {
                        if self.get(x + i, y + wall.height) != Cell::Filled {
                            break 'joe;
                        }
                    }
                    for i in 0..wall.width {
                        self.set(x + i, y + wall.height, Cell::Colored(color));
                    }
                    wall.height += 1;
                }
                self.walls.push(wall);
            }
        }
    }
    fn display(&mut self) {
        let blocks = ['🟪', '🟩', '🟨', '🟥', '🟫', '🟧', '🟦', '⬛'];
        let mut output = String::new();
        for (i, cell) in self.map.iter().enumerate() {
            if i as u32 % self.size == 0 {
                println!("{}", output);
                output = String::new();
            }
            output.push(match *cell {
                Cell::Blank | Cell::ConnectedBlank => '⬜',
                Cell::Colored(color) => blocks[color as usize % 8],
                Cell::Filled => panic!("uncolored wall found"),
            });
        }
        println!("{}", output);
        println!("The seed is: {}", self.maze_seed)
    }
}
impl SeedMaze {
    fn new(config: Config) -> Self {
        let Config { size, maze_seed, seed_range, mut prng, turn_chance, termination_chance, wall_wrapping } = config;
        let seed_amount = prng.next_int(seed_range.max - seed_range.min) + seed_range.min;
        let seeds = Vec::new();
        let map = SquareMap::new(size, maze_seed);

        Self { prng, seed_amount, turn_chance, termination_chance, wall_wrapping, seeds, map }
    }
    fn init(&mut self) {
        self.seed_walls();
        self.grow_walls();
        self.sprinkle_walls();
        self.find_pockets();
        self.map.replace(Cell::Blank, Cell::Filled);
        self.map.combine();
        self.map.display()
    }
    fn seed_walls(&mut self) {
        for _ in 0..10000 {
            if self.seeds.len() >= self.seed_amount as usize {
                break;
            }
            let loc = Seed {
                x: self.prng.next_int(self.map.size).saturating_sub(1),
                y: self.prng.next_int(self.map.size).saturating_sub(1),
            };
            if self.seeds.iter().any(|r| loc.x.abs_diff(r.x) <= 3 && loc.y.abs_diff(r.y) <= 3) {
                continue;
            }
            if loc.x == 0 || loc.y == 0 || loc.x >= self.map.size - 1 || loc.y >= self.map.size - 1 {
                continue;
            }
            self.map.set(loc.x, loc.y, Cell::Filled);
            self.seeds.push(loc);
        }
    }
    fn sprinkle_walls(&mut self) {
        for _ in 0..15 { 
            let loc = Seed {
                x: self.prng.next_int(self.map.size).wrapping_sub(1),
                y: self.prng.next_int(self.map.size).wrapping_sub(1),
            };
            if self.seeds.iter().any(|r| loc.x.abs_diff(r.x) <= 2 && loc.y.abs_diff(r.y) <= 2) {
                continue;
            }
            if loc.x >= self.map.size || loc.y >= self.map.size {
                continue;
            }
            self.map.set(loc.x, loc.y, Cell::Filled);
        }
    }
    fn grow_walls(&mut self) {
        let direction = [
            [-1, 0], [1, 0], // left and right
            [0, -1], [0, 1], // up and down
        ];
        for seed in &mut self.seeds {
            let mut dir = direction[self.prng.next_int(4) as usize];
            let mut termination: f64 = 1.0;
            while termination >= self.termination_chance {
                termination = self.prng.next_float();
                let [x, y] = dir;
                seed.x = seed.x.wrapping_add_signed(x);
                seed.y = seed.y.wrapping_add_signed(y);
                if seed.x == 0 || seed.y == 0 || seed.x >= self.map.size - 1 || seed.y >= self.map.size - 1 {
                    break;
                }
                if self.wall_wrapping {
                    seed.x = if seed.x == 0 { self.map.size } else if seed.x == self.map.size { 1 } else { seed.x };
                    seed.y = if seed.y == 0 { self.map.size } else if seed.y == self.map.size { 1 } else { seed.y };
                }
                self.map.set(seed.x, seed.y, Cell::Filled);
                    self.prng.next_float();
                if self.prng.next_float() <= self.turn_chance {
                    // seeds_len += 1;
                    // dir = perpendicular(dir)[self.prng.next_int(2) as usize];
                    dir = *direction.iter().filter(|a| **a != dir && **a != [-dir[0], -dir[1]]).collect::<Vec<_>>()[self.prng.next_int(2) as usize];
                    // self.map.set(seed.x.wrapping_add_signed(x), seed.y.wrapping_add_signed(y), Cell::Filled);
                }
            }
        }
    }
    fn find_pockets(&mut self) {
        let mut queue: Vec<[u32; 2]> = vec![[0, 0]];
        self.map.set(0, 0, Cell::ConnectedBlank);
        let mut checked_indices = HashSet::new();
        checked_indices.insert(0);
        while !queue.is_empty() {
            let [x, y] = queue.remove(0);
            let sides = [
                [x.wrapping_sub(1), y], // left
                [x + 1, y], // right
                [x, y.wrapping_sub(1)], // top
                [x, y + 1], // bottom
            ];
            for [nx, ny] in sides {
                if !(nx < self.map.size && ny < self.map.size) {
                    continue;
                }
                if self.map.get(nx, ny) != Cell::Blank {
                    continue;
                }
                let i = ny * self.map.size + nx;
                if checked_indices.contains(&i) {
                    continue;
                }
                checked_indices.insert(i);
                queue.push([nx, ny]);
                self.map.set(nx, ny, Cell::ConnectedBlank);
            }
        }
    }
}

fn main() {
    let config: Config = Config {
        size: 40,
        maze_seed: parse_seed("owo"),
        prng: SeededGenerator::new(parse_seed("owo")),
        seed_range: SeedRange { min: 30, max: 50 },
        turn_chance: 0.17,
        termination_chance: 0.12,
        wall_wrapping: true,
    };

    let mut c = SeedMaze::new(config);
    c.init();
}
