use tcod::colors::*;
use tcod::console::*;
use std::cmp;
use rand::Rng;

// actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

// size of the map
const MAP_WIDTH: i32 = 80 ;
const MAP_HEIGHT: i32 = 45;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const COLOR_DARK_WALL: Color = Color {r:0, g:0, b:100};
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};

type Map = Vec<Vec<Tile>>;

struct Game {
    map: Map,
}

/// Map tile and its properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
}

impl Tile {

    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
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
}

impl Object {
    pub fn new (x: i32, y: i32, char: char, color: Color) -> Self {
        Object { x, y, char, color }
    }

    // movement
    pub fn move_by(& mut self, dx: i32, dy: i32, game: &Game) {

        if !game.map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    // rendering the Object
    pub fn draw(&self,  con: &mut dyn Console) {
        con.set_default_background(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
}

/// A rectangle on the map , used to render a room
///
#[derive(Clone,Copy,Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y:i32, w: i32, h:i32 ) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x+w,
            y2: y+h,
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
}

fn create_h_tunnel(x1: i32, x2: i32,y: i32,map: &mut Map){
    // making a tunnel
    for x in cmp::min(x1, x2)..(cmp::max(x1,x2) + 1){
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x:i32, map: &mut Map){
    // making the tunnel
    for y in cmp::min(y1,y2)..(cmp::max(y1, y2)+1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn make_map() -> Map {
    // blocked tiles filled
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range( ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1 );
        let h = rand::thread_rng().gen_range( ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1 );

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

            create_room(new_room, &mut map)
        }
    }

    map
}

fn render_all(tcod: &mut Tcod, game : &Game, objects: &[Object]) {
    // Traverse and set the tile color
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let wall = game.map[x as usize][y as usize].block_sight;
            if wall {
                tcod.con
                        .set_char_background(x,y,COLOR_DARK_WALL,BackgroundFlag::Set);
            }
            else {
                tcod.con
                        .set_char_background(x,y,COLOR_DARK_GROUND,BackgroundFlag::Set);
            }
        }
    }

    // draw all objects
    for object in objects {
        object.draw( &mut tcod.con);
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
fn handle_keys(tcod: &mut Tcod,game: &Game, player: &mut Object ) -> bool {
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
        Key { code: Up, .. } => player.move_by(0, -1,game),
        Key { code: Down, .. } => player.move_by(0, 1,game),
        Key { code: Left, .. } => player.move_by(-1, 0,game),
        Key { code: Right, .. } => player.move_by(1, 0,game),

        _ => {}
    }

    false
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    let con = Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT);

    let mut tcod = Tcod { root , con };

    // create player
    let player = Object::new(25, 23, '@', WHITE);

    // create an Non-player character
    let npc = Object::new(SCREEN_WIDTH/2 -5 , SCREEN_HEIGHT/2, '+', RED);

    // list with all objects in the game
    let mut objects = [player,npc];

    let game = Game {
        // make the map - not rendered though
        map: make_map(),
    };

    while !tcod.root.window_closed() {

        // Clear previous frame
        tcod.con.clear();

        // render
        render_all(&mut tcod, &game, &objects);

        tcod.root.flush();

        // handle keys and exit game if needed
        let player = &mut objects[0];
        let exit = handle_keys(&mut tcod, &game, player);

        if exit {
            break ;
        }
    }
}
