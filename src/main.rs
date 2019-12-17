use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};
use PlayerAction::*;

// actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

// size of the map
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;


//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const MAX_ROOM_MONSTERS: i32 = 3;

// sizes and coordinates relevant for the GUI
const BAR_WIDTH: i32 = 10;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

//FOV algorithm consts
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic; // Default FOV algorithm
const FOV_LIGHT_WALLS: bool = true; // light walls or not
const TORCH_RADIUS: i32 = 10;

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

#[derive(Clone, Copy, PartialEq)]
enum PlayerAction { 
    TookTurn, 
    DidntTakeTurn,
    Exit,
}

fn player_death(player: &mut Object) {
    // player has died
    println!("You died!!");

    // transform into corpse!
    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Object) { 
    println!("{} is dead! ", monster.name);
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
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

// combat-related properties and methods (for player and various monsters)
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
    max_hp: i32, 
    hp: i32, 
    defence: i32,
    power: i32,
    on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object){
        use DeathCallback::*;
        let callback: fn(&mut Object) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Ai {
    Basic, 
}


fn ai_take_turn(monster_id : usize, tcod: &Tcod, game: &Game, objects: &mut [Object]) {
    // monster's turn

    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x,monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        }
        // close enough to attack the PLAYEr
        else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player);
        }
    }
}

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
    fighter: Option<Fighter>,
    ai: Option<Ai>,
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
            fighter: None,
            ai: None, 
        }
    }

    // getting and settings the position of the object
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

    pub fn distance_to(&self, other: &Object) -> f32 { 
        let dx = other.x - self.x ; 
        let dy = other.y - self.y ;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32) {
        // if possible, cause damage
        if let Some(fighter) = self.fighter.as_mut() { 
            if damage > 0 {
                fighter.hp -= damage ; 
            }
        }

        // check for death, call the on_death
        if let Some(fighter) = self.fighter { 
            if fighter.hp <= 0 {
                self.alive = false; 
                fighter.on_death.callback(self);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object){
        // simple attack formula
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defence);
        
        // reflect the damage
        if damage > 0 { 
            // PLAYER takes some damage
            println!(
                "{} attacks {} for {} damage." ,
                self.name, target.name, damage
            );
            target.take_damage(damage);
        }
        else
        {
            println!(
                "{} attacks {} to no effect",
                self.name, target.name
            );
        }
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
    panel: Offscreen,
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

// movement
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x,y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // distance from object to the target
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    // we normalize this to length 1, round it and convert to an integer

    let dx = (dx as f32 / distance).round() as i32; 
    let dy = (dy as f32 / distance).round() as i32;

    move_by(id, dx, dy, map, objects);
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

fn player_move_or_attack(dx: i32, dy: i32, game: &Game, objects: &mut [Object]) { 
    // the coordinates the player is moving to / attacking 
    let x = objects[PLAYER].x + dx ; 
    let y = objects[PLAYER].y + dy ;

    // try to find an attackable object
    let target_id = objects.
        iter()
        .position( | object | object.fighter.is_some() &&  object.pos() == (x,y));

    match target_id {
        Some(target_id) => {
            let (player,target) = mut_two(PLAYER, target_id, objects);
            player.attack(target);
        }
        None => { 
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}


fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    // choosing random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // choosing a random spot for the monstor
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {

            let mut monster = if rand::random::<f32>() < 0.6 {
                // 60% change of getting an orc
                // creating an orc
                let mut orc = Object::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter {
                    max_hp: 10,
                    hp: 10,
                    defence: 0, 
                    power: 3,
                    on_death: DeathCallback::Monster,
                });
                orc.ai = Some(Ai::Basic);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', "troll", DARKER_GREEN, true);
                
                troll.fighter = Some( Fighter { 
                    max_hp: 16, 
                    hp: 16,
                    defence: 1,
                    power: 4,
                    on_death: DeathCallback::Monster,
                });
                troll.ai = Some(Ai::Basic);
                troll
            };
            
            monster.alive = true;
            objects.push(monster);
        }
    }
}
/*
    To avoid ownership issues, we splice the items into two slices
    panics if the indexes are equal
*/
fn mut_two<T>(index1: usize, index2: usize, items: &mut[T]) -> (&mut T, &mut T) {
    assert!(index1 != index2);
    let split_at_index = cmp::max(index1, index2);
    let(first_slice, second_slice) = items.split_at_mut(split_at_index);

   if index1 < index2 {
       (&mut first_slice[index1], &mut second_slice[0])
   } else {
       (&mut second_slice[0], &mut first_slice[index2])
   }
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
            place_objects(new_room, &map, objects);

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
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y))
        .collect();
    
    // sort so that non-blocking objects come first 
    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    
    // draw the objects
    for object in &to_draw {
            object.draw(&mut tcod.con);
    }

    // blit the contents of "con" to the root console and render
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );

    // showing the stats
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // show the player stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);

    render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_RED, DARKER_RED);

    // blit the contents of `panel` to the root console
    blit(
        &tcod.panel,
        (0, 0),
        (SCREEN_WIDTH, PANEL_HEIGHT),
        &mut tcod.root,
        (0, PANEL_Y),
        1.0,
        1.0,
    );
}

fn render_bar( panel: &mut Offscreen, x: i32, y: i32, total_width: i32,
            name: &str, value: i32, maximum: i32, bar_color: Color, back_color: Color,) {

                // rendering the bar with HP and XP. 
                let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32 ;

                // rendering the background
                panel.set_default_background(back_color);
                panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

                // rendering the top bar
                panel.set_default_background(bar_color);
                if bar_width > 0 {
                    panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
                }

                // centered text with values
                panel.set_default_foreground(WHITE);
                panel.print_ex(x+total_width /2 , y, BackgroundFlag::None, TextAlignment::Center,
                     &format!("{} : {}/{}", name, value, maximum),
                );
}

// define the behaviour of the keys for control
fn handle_keys(tcod: &mut Tcod, game: &Game, objects: &mut Vec<Object>) -> PlayerAction {
    // TODO: handle keys
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    match (key, key.text(), player_alive) {

        ( 
            Key {
            code: Enter,
            alt: true,
            ..
            },
            _,
            _,
        ) => {
            // Alt + Enter : toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }

        (Key { code: Escape, .. },_,_,) => Exit, // exit game

        // movement keys
        (Key { code: Up, .. }, _, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        (Key { code: Down, .. },_,true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        },
        (Key { code: Left, .. },_,true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        (Key { code: Right, .. },_,true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }

        _ => DidntTakeTurn,
    }
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("RogueMax")
        .init();

    let mut tcod = Tcod {
        root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    // create player
    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true ;
    player.fighter = Some( Fighter {
        max_hp: 30, 
        hp: 30, 
        defence: 2, 
        power: 5,
        on_death: DeathCallback::Player,
    });

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
        previous_player_pos = objects[PLAYER].pos();
        let player_action = handle_keys(&mut tcod, &game, &mut objects);
        if player_action == PlayerAction::Exit {
            break;
        }

        // monsters turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &game, &mut objects);
                }
            }
        }
    }
}
