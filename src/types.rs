use std::cmp::min;

use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::Mouse;
use tcod::colors::*;

use crate::constants::*;



pub struct Messages(pub Vec<(String, Color)>);

impl Messages {
    pub fn new() -> Messages {
        Messages(Vec::new())
    }

    pub fn message<T: Into<String>>(&mut self, message: T, color: Color) {
        if self.0.len() == MSG_HEIGHT {
            self.0.remove(0);
        }

        self.0.push((message.into(), color));
    }
}


pub struct Game {
    pub root: Root,
    pub console: Offscreen,
    pub fov: FovMap,
    pub mouse: Mouse,
    pub panel: Offscreen,
}

impl Game {
    pub fn with_root(root: Root) -> Game {
        Game {
            root: root,
            console: Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
            mouse: Default::default(),
            panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
    pub tile_type: TileType,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { blocked: false,
               block_sight: false,
               explored: false,
               tile_type: TileType::Empty
        }
    }

    pub fn wall() -> Self {
        Tile { blocked: true,
               block_sight: true,
               explored: false,
               tile_type: TileType::Wall,
        }
    }

    pub fn water() -> Self {
        Tile { blocked: true,
               block_sight: false,
               explored: false,
               tile_type: TileType::Water,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TileType {
    Empty,
    Wall,
    Water
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Obstacle {
    Block,
    Wall,
    Square,
    LShape,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ai;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Heal,
}

pub enum UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeathCallback {
    Player,
    Monster,
}

#[derive(Clone, Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
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
            item: None,        
        }
    }

    pub fn draw(&self, console: &mut Console) {
        console.set_default_foreground(self.color);
        console.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn clear(&self, console: &mut Console) {
        console.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, messages: &mut Messages) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            messages.message(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), WHITE);
            target.take_damage(damage);
        } else {
            messages.message(format!("{} attacks {} but it has no effect!", self.name, target.name), WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect  {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x1: x, y1: y, x2: x + w, y2: y + h }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2) &&
        (self.x2 >= other.x1) &&
        (self.y1 <= other.y2) &&
        (self.y2 >= other.y1)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Position(pub i32, pub i32);

impl Position {
    pub fn distance(&self, other: &Position) -> i32 {
        let dist_i32 = (self.0 - other.0).pow(2) + (self.1 - other.1).pow(2);
        (dist_i32 as f64).sqrt() as i32
    }
}

