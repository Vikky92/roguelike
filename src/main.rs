use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};
// actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

//FOV algorithm consts
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic; // Default FOV algorithm
const FOV_LIGHT_WALLS: bool = true; // light walls or not
const TORCH_RADIUS: i32 = 10;

// size of the map
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const MAX_ROOM_MONSTERS: i32 = 3;

// player is first object
const PLAYER: usize = 0;
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

type Map = Vec<Vec<Tile>>;

struct Game {
    map: Map,
}

/// Map tile and its properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    explored: bool,
    block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            explored: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            explored: false,
            block_sight: true,
        }
    }
}

const LIMIT_FPS: i32 = 20; // 20 frames-per-second maximum

/// This template Object can be used for multiple items in the game..
/// It is represented by a character on the screen
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String, 
    blocks: bool,
    alive: bool,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x: x, 
            y: y, 
            char: char, 
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
        }
    }

    // movement
    pub fn move_by(&mut self, dx: i32, dy: i32, game: &Game) {
        if !game.map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    // getting and settings the postion of the object
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    // rendering the Object
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_background(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
}

/// A rectangle on the map , used to render a room
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        // returns true if this rectangle intersects with another one
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn create_room(room: Rect, map: &mut Map) {
    // go through the tile and make them empty
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    // making a tunnel
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    // making the tunnel
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn place_objects(room: Rect, objects: &mut Vec<Object>) {
    // choosing random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // choosing a random spot for the monstor
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        let mut monster = if rand::random::<f32>() < 0.6 {
            // 60% change of getting an orc

            // creating an orc
            Object::new(x, y, 'o', colors::DESATURATED_GREEN)
        } else {
            Object::new(x, y, 'T', colors::DARKER_GREEN)
        };

        objects.push(monster);
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // check if tile is blocked
    if map[x as usize][y as usize].blocked {
        return true;
    }

    // now check for any blocked objects
    objects
        .iter()
        .any( | object | object.blocks && object.pos() == (x,y))
        
}

fn make_map(objects: &mut Vec<Object>) -> Map {
    // blocked tiles filled
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // Checking intersection with other rooms
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            // going ahead and creating the room
            create_room(new_room, &mut map);

            // adding characters to the new_room
            place_objects(new_room, objects);

            // center coordinates of the room
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // as this is the first room, player from the center of this room
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                // connect all the other rooms with a tunnel

                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // randomize the tunnel generation

                if rand::random() {
                    // move horizontally and then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }

            rooms.push(new_room);
        }
    }

    map
}

// main render program
fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if needed ( the player moved or something)
        let player = &objects[PLAYER];
        tcod.fov
            .compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }
    // Traverse and set the tile color
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);

            let wall = game.map[x as usize][y as usize].block_sight;
            let color = match (visible, wall) {
                // outside FOV:
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                // inside FOV
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            let explored = &mut game.map[x as usize][y as usize].explored;

            if visible {
                // as its visible, it means that it has been explored
                *explored = true;
            }

            if *explored {
                tcod.con
                    .set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }

    // draw all objects
    for object in objects {
        object.draw(&mut tcod.con);
    }

    // blit the contents of "con" to the root console and render
    blit(
        &tcod.con,
        (0, 0),
        (SCREEN_WIDTH, SCREEN_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}
// define the behaviour of the keys for control
fn handle_keys(tcod: &mut Tcod, game: &Game, player: &mut Object) -> bool {
    // TODO: handle keys
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);

    match key {
        Key {
            code: Enter,
            alt: true,
            ..
        } => {
            // Alt + Enter : toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
        }

        Key { code: Escape, .. } => return true, // exit game

        // movement keys
        Key { code: Up, .. } => player.move_by(0, -1, game),
        Key { code: Down, .. } => player.move_by(0, 1, game),
        Key { code: Left, .. } => player.move_by(-1, 0, game),
        Key { code: Right, .. } => player.move_by(1, 0, game),

        _ => {}
    }

    false
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(MAP_WIDTH, MAP_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    let mut tcod = Tcod {
        root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    // create player
    let player = Object::new(0, 0, '@', WHITE);

    // list with all objects in the game
    let mut objects = vec![player];

    let mut game = Game {
        // make the map - not rendered though
        map: make_map(&mut objects),
    };

    // first time FOV recomputation
    let mut previous_player_pos = (-1, -1);

    while !tcod.root.window_closed() {
        // Clear previous frame
        tcod.con.clear();

        // render
        let fov_recompute = previous_player_pos != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(&mut tcod, &mut game, &objects, fov_recompute);

        tcod.root.flush();

        // handle keys and exit game if needed
        let player = &mut objects[0];
        previous_player_pos = (player.x, player.y);
        let exit = handle_keys(&mut tcod, &game, player);

        if exit {
            break;
        }
    }
}
