use std::iter::Iterator;
use std::fmt;

use euclid::*;

use serde::{Serialize, Deserialize};

use crate::constants::*;
use crate::types::*;
use crate::utils::*;
use crate::map::{Wall, Blocked};
use crate::ai::Behavior;


#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum Action {
    Move(Movement),
    StateChange(Behavior),
    Pickup(EntityId),
    ThrowItem(Pos, EntityId), // end position, item id
    Pass,
    Yell,
    UseItem(Pos), // item used towards position, or just player pos
    ArmDisarmTrap(EntityId),
    PlaceTrap(Pos, EntityId), // position to place, trap id
    // TODO consider just using Option<Action> instead
    NoAction,
}

impl Action {
    pub fn none() -> Action {
        return Action::NoAction;
    }

    pub fn is_none(&self) -> bool {
        return *self == Action::NoAction;
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoveMode {
    Sneak,
    Walk,
    Run,
}

impl Default for MoveMode {
    fn default() -> MoveMode {
        return MoveMode::Walk;
    }
}

impl MoveMode {
    pub fn increase(&self) -> MoveMode {
        match self {
            MoveMode::Sneak => MoveMode::Walk,
            MoveMode::Walk => MoveMode::Run,
            MoveMode::Run => MoveMode::Run,
        }
    }

    pub fn decrease(&self) -> MoveMode {
        match self {
            MoveMode::Sneak => MoveMode::Sneak,
            MoveMode::Walk => MoveMode::Sneak,
            MoveMode::Run => MoveMode::Walk,
        }
    }
}

impl fmt::Display for MoveMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MoveMode::Sneak => write!(f, "sneaking"),
            MoveMode::Walk => write!(f, "walking"),
            MoveMode::Run => write!(f, "running"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Attack {
    Attack(EntityId), // target_id
    Push(EntityId, Pos), //target_id, delta_pos
    Stab(EntityId), // target_id
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MoveType {
    Move,
    Pass,
    JumpWall,
    WallKick(i32, i32),
    Collide,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Movement {
    pub pos: Pos,
    pub typ: MoveType,
    pub attack: Option<Attack>,
}

impl Default for Movement {
    fn default() -> Movement {
        return Movement {
            pos: Pos::new(0, 0),
            typ: MoveType::Pass,
            attack: None,
        };
    }
}

impl Movement {
    pub fn new(pos: Pos, typ: MoveType, attack: Option<Attack>) -> Movement {
        return Movement {
            pos,
            typ,
            attack,
        };
    }

    pub fn move_to(pos: Pos, typ: MoveType) -> Movement {
        return Movement {
            pos,
            typ,
            attack: None,
        };
    }

    pub fn attack(pos: Pos, typ: MoveType, attack: Attack) -> Movement {
        return Movement {
            pos,
            typ,
            attack: Some(attack),
        };
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum Cardinal {
    Up,
    Down,
    Left,
    Right
}

impl Cardinal {
    pub fn from_dxy(last: Option<Cardinal>, dx: i32, dy: i32) -> Option<Cardinal> {
        if dx == 0 && dy == 0 {
            None
        } else if dx == 0 && dy < 0 {
            Some(Cardinal::Up)
        } else if dx == 0 && dy > 0 {
            Some(Cardinal::Down)
        } else if dx > 0 && dy == 0 {
            Some(Cardinal::Right)
        } else if dx < 0 && dy == 0 {
            Some(Cardinal::Left)
        } else {
            if let Some(_dir) = last {
                if dx > 0 && dy > 0 {
                    Some(Cardinal::Right)
                } else if dx > 0 && dy < 0 {
                    Some(Cardinal::Right)
                } else if dx < 0 && dy > 0 {
                    Some(Cardinal::Left)
                } else if dx < 0 && dy < 0 {
                    Some(Cardinal::Left)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    pub fn x_signum(&self) -> i32 {
        match self {
            Cardinal::Up => 0,
            Cardinal::Down => 0,
            Cardinal::Left => -1,
            Cardinal::Right => 1,
        }
    }

    pub fn y_signum(&self) -> i32 {
        match self {
            Cardinal::Up => -1,
            Cardinal::Down => 1,
            Cardinal::Left => 0,
            Cardinal::Right => 0,
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
    DownLeft,
    DownRight,
    UpLeft,
    UpRight,
}

impl Direction {
    pub fn from_dxy(dx: i32, dy: i32) -> Option<Direction> {
        if dx == 0 && dy == 0 {
            None
        } else if dx == 0 && dy < 0 {
            Some(Direction::Up)
        } else if dx == 0 && dy > 0 {
            Some(Direction::Down)
        } else if dx > 0 && dy == 0 {
            Some(Direction::Right)
        } else if dx < 0 && dy == 0 {
            Some(Direction::Left)
        } else if dx > 0 && dy > 0 {
            Some(Direction::DownRight)
        } else if dx > 0 && dy < 0 {
            Some(Direction::UpRight)
        } else if dx < 0 && dy > 0 {
            Some(Direction::DownLeft)
        } else if dx < 0 && dy < 0 {
            Some(Direction::UpLeft)
        } else {
            panic!(format!("Direction should not exist {:?}", (dx, dy)));
        }
    }

    pub fn horiz(self) -> bool {
        match self {
            Direction::Left | Direction::Right |
            Direction::Up | Direction::Down => true,
            _ => false,
        }
    }

    pub fn diag(self) -> bool {
        match self {
            Direction::DownLeft | Direction::DownRight |
            Direction::UpLeft   | Direction::UpRight => true,
            _ => false,
        }
    }

    pub fn into_move(self) -> (i32, i32) {
        match self {
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::DownLeft => (-1, 1),
            Direction::DownRight => (1, 1),
            Direction::UpLeft => (-1, -1),
            Direction::UpRight => (1, -1),
        }
    }

    pub fn move_actions() -> Vec<Direction> {
        return vec!(Direction::Left,
                    Direction::Right,
                    Direction::Up,
                    Direction::Down,
                    Direction::DownLeft,
                    Direction::DownRight,
                    Direction::UpLeft,
                    Direction::UpRight);
    }

    pub fn from_f32(flt: f32) -> Direction {
        let index = (flt * 8.0) as usize;
        let dirs = Direction::move_actions();
        return dirs[index];
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Reach {
    Single(usize),
    Diag(usize),
    Horiz(usize),
}

impl Reach {
    pub fn new() -> Reach {
        return Reach::Single(1);
    }

    pub fn single(dist: usize) -> Reach {
        return Reach::Single(dist);
    }

    pub fn diag(dist: usize) -> Reach {
        return Reach::Single(dist);
    }

    pub fn horiz(dist: usize) -> Reach {
        return Reach::Single(dist);
    }

    pub fn dist(&self) -> usize {
        match self {
            Reach::Single(dist) => *dist,
            Reach::Diag(dist) => *dist,
            Reach::Horiz(dist) => *dist,
        }
    }

    pub fn with_dist(&self, dist: usize) -> Reach {
        match self {
            Reach::Single(_) => Reach::Single(dist),
            Reach::Diag(_) => Reach::Diag(dist),
            Reach::Horiz(_) => Reach::Horiz(dist),
        }
    }

    pub fn closest_to(&self, pos: Pos, other: Pos) -> Pos {
        let offsets = self.offsets();

        let mut closest: Pos = *offsets.get(0).expect(&format!("Reach had 0 options {:?}?", self));

        for offset in offsets {
            let other_pos = add_pos(pos, offset);
            if distance(other, other_pos) < distance(other, closest) {
                closest = other_pos;
            }
        }

        return closest;
    }

    pub fn attacks_with_reach(&self, move_action: &Direction) -> Vec<Pos> {
        let mut positions = Vec::new();

        if let Some(pos) = self.move_with_reach(move_action) {
            for pos in line_inclusive(Pos::new(0, 0), pos) {
                positions.push(Pos::from(pos));
            }
        }

        return positions;
    }

    pub fn move_with_reach(&self, move_action: &Direction) -> Option<Pos> {
        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    Direction::Left => Some(Pos::new(neg_dist, 0)),
                    Direction::Right => Some(Pos::new(dist, 0)),
                    Direction::Up => Some(Pos::new(0, neg_dist)),
                    Direction::Down => Some(Pos::new(0, dist)),
                    Direction::DownLeft => Some(Pos::new(neg_dist, dist)),
                    Direction::DownRight => Some(Pos::new(dist, dist)),
                    Direction::UpLeft => Some(Pos::new(neg_dist, neg_dist)),
                    Direction::UpRight => Some(Pos::new(dist, neg_dist)),
                }
            }

            Reach::Diag(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    Direction::Left => None,
                    Direction::Right => None,
                    Direction::Up => None,
                    Direction::Down => None,
                    Direction::DownLeft => Some(Pos::new(neg_dist, dist)),
                    Direction::DownRight => Some(Pos::new(dist, dist)),
                    Direction::UpLeft => Some(Pos::new(neg_dist, neg_dist)),
                    Direction::UpRight => Some(Pos::new(dist, neg_dist)),
                }
            }

            Reach::Horiz(dist) => {
                let dist = (*dist) as i32;
                let neg_dist = dist * -1;
                match move_action {
                    Direction::Left => Some(Pos::new(neg_dist, 0)),
                    Direction::Right => Some(Pos::new(dist, 0)),
                    Direction::Up => Some(Pos::new(0, neg_dist)),
                    Direction::Down => Some(Pos::new(0, dist)),
                    Direction::DownLeft => None,
                    Direction::DownRight => None,
                    Direction::UpLeft => None,
                    Direction::UpRight => None,
                }
            }
        }
    }

    pub fn reachables(&self, start: Pos) -> Vec<Pos> {
        let offsets = self.offsets();
        return offsets.iter()
                      .map(|off| add_pos(start, *off))
                      .collect::<Vec<Pos>>();
    }

    pub fn offsets(&self) -> Vec<Pos> {
        let end_points: Vec<Pos>;

        match self {
            Reach::Single(dist) => {
                let dist = (*dist) as i32;
                let offsets =
                    vec!( (0, dist),      (-dist, dist), (-dist,  0),
                          (-dist, -dist), (0,  -dist),   (dist, -dist),
                          (dist,  0), (dist, dist));
                end_points = offsets.iter().map(|pair| Pos::from(*pair)).collect();
            },

            Reach::Horiz(dist) => {
                let dist = (*dist) as i32;
                let mut offsets = vec!();
                for dist in 1..=dist {
                    offsets.push((dist, 0));
                    offsets.push((0, dist));
                    offsets.push((-1 * dist, 0));
                    offsets.push((0, -1 * dist));
                }
                end_points = offsets.iter().map(|pair| Pos::from(*pair)).collect();
            },


            Reach::Diag(dist) => {
                let mut offsets = vec!();
                let dist = (*dist) as i32;
                for dist in 1..=dist {
                    offsets.push((dist, dist));
                    offsets.push((-1 * dist, dist));
                    offsets.push((dist, -1 * dist));
                    offsets.push((-1 * dist, -1 * dist));
                }
                end_points = offsets.iter().map(|pair| Pos::from(*pair)).collect();
            },
        }

        let mut offsets = Vec::new();
        for end in end_points {
            for pos in line_inclusive(Pos::new(0, 0), end) {
                offsets.push(Pos::from(pos));
            }
        }

        return offsets;
    }
}

#[test]
pub fn test_reach_offsets_horiz() {
    let horiz = Reach::Horiz(1);
    let offsets = horiz.offsets();

    let expected_pos =
        vec!((1, 0), (-1, 0), (0, 1), (0, -1)).iter()
                                              .map(|p| Pos::from(*p))
                                              .collect::<Vec<Pos>>();
    assert!(offsets.iter().all(|p| expected_pos.iter().any(|other| other == p)));
}

#[test]
pub fn test_reach_offsets_diag() {
    let horiz = Reach::Diag(1);
    let offsets = horiz.offsets();

    let expected_pos =
        vec!((1, 1), (-1, 1), (1, -1), (-1, -1)).iter()
                                              .map(|p| Pos::from(*p))
                                              .collect::<Vec<Pos>>();
    assert!(offsets.iter().all(|p| expected_pos.iter().any(|other| other == p)));
}

#[test]
pub fn test_reach_offsets_single() {
    let horiz = Reach::Single(1);
    let offsets = horiz.offsets();

    let expected_pos_vec =
        vec!((1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1));

    let expected_pos = expected_pos_vec.iter()
                                       .map(|p| Pos::from(*p))
                                       .collect::<Vec<Pos>>();

    assert!(offsets.iter().all(|p| expected_pos.iter().any(|other| other == p)));
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Momentum {
    pub mx: i32,
    pub my: i32,
    pub max: i32,
}

impl Default for Momentum {
    fn default() -> Momentum {
        Momentum {
            mx: 0,
            my: 0,
            max: MAX_MOMENTUM,
        }
    }
}

impl Momentum {
    pub fn running(&mut self) -> bool {
        return self.magnitude() != 0;
    }

    pub fn at_maximum(&self) -> bool {
        return self.magnitude() == MAX_MOMENTUM;
    }
        
    pub fn magnitude(&self) -> i32 {
        if self.mx.abs() > self.my.abs() {
            return self.mx.abs();
        } else {
            return self.my.abs();
        }
    }

    pub fn diagonal(&self) -> bool {
        return self.mx.abs() != 0 && self.my.abs() != 0;
    }

    pub fn moved(&mut self, dx: i32, dy: i32) {
        // if the movement is in the opposite direction, and we have some momentum
        // currently, lose our momentum.

        if self.mx != 0 && dx.signum() != self.mx.signum() {
            self.mx = 0;
        } else {
            self.mx = clamp(self.mx + dx.signum(), -self.max, self.max);
        }

        if self.my != 0 && dy.signum() != self.my.signum() {
            self.my = 0;
        } else {
            self.my = clamp(self.my + dy.signum(), -self.max, self.max);
        }
    }

    pub fn set_momentum(&mut self, mx: i32, my: i32) {
        self.mx = mx;
        self.my = my;
    }

    pub fn along(&self, dx: i32, dy: i32) -> bool {
        return (self.mx * dx + self.my * dy) > 0;
    }

    pub fn clear(&mut self) {
        self.mx = 0;
        self.my = 0;
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveResult {
    entity: Option<EntityId>,
    blocked: Option<Blocked>,
    move_pos: Pos,
}

impl MoveResult {
    pub fn with_pos(pos: Pos) -> MoveResult {
        return MoveResult {
            entity: None,
            blocked: None,
            move_pos: pos
        };
    }

    pub fn no_collision(&self) -> bool {
        return self.blocked.is_none() && self.entity.is_none();
    }
}

// Moves the given object with a given offset, returning the square that it collides with, or None
/// indicating no collision.
/// NOTE if the movement carries multiple tiles, then the resulting MoveResult can report that
/// there is a blocking wall, and on a different location a blocking entity. These are checked
/// separately.
pub fn check_collision(pos: Pos,
                       dx: i32,
                       dy: i32,
                       data: &GameData) -> MoveResult {
    let mut last_pos = pos;
    let mut result: MoveResult =
        MoveResult::with_pos(pos + Vector2D::new(dx, dy));

    // if no movement occurs, no need to check walls and entities.
    if !(dx == 0 && dy == 0) {
        if let Some(blocked) = data.map.is_blocked_by_wall(pos, dx, dy) {
            result.blocked = Some(blocked);
            result.move_pos = blocked.start_pos;
        } 

        // check for collision with an enitity
        let move_line = line_inclusive(pos, Pos::new(pos.x + dx, pos.y + dy));

        for line_tuple in move_line {
            let line_pos = Pos::from(line_tuple);

            if let Some(key) = data.has_blocking_entity(line_pos) {
                result.move_pos = last_pos;
                result.entity = Some(key);
                break;
            }

            // if we are blocked by a wall, and the current position is at that blocked
            // position, we don't need to continue the search
            if let Some(blocked) = result.blocked {
                if line_pos == blocked.start_pos {
                    break;
                }
            }

            last_pos = pos;
        }
    }

    return result;
}

pub fn entity_move_not_blocked(entity_id: EntityId, move_pos: Pos, delta_pos: Pos, data: &GameData) -> Option<Movement> {
    let movement: Option<Movement>;

    let pos = data.entities.pos[&entity_id];

    let next_pos = next_pos(pos, delta_pos);
    if let Some(other_id) = data.has_blocking_entity(next_pos) {
        if can_stab(data, entity_id, other_id) {
           let attack = Attack::Stab(other_id);
           movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
       } else {
          movement = Some(Movement::move_to(move_pos, MoveType::Move));
       }
    } else {
      movement = Some(Movement::move_to(move_pos, MoveType::Move));
    }

    return movement;
}

pub fn entity_move_blocked_by_wall(entity_id: EntityId, delta_pos: Pos, blocked: &Blocked, data: &GameData) -> Option<Movement> {
    let mut movement: Option<Movement>;

    let pos = data.entities.pos[&entity_id];
    let mut jumped_wall = false;

    if data.entities.move_mode[&entity_id] == MoveMode::Run {
        if !blocked.blocked_tile && blocked.wall_type == Wall::ShortWall {
            jumped_wall = true;
        } 
    }

    if jumped_wall {
        // TODO remove when confirmed that this is not desired
        // if we jump the wall, we have to recheck for collisions for the
        // remaining move distance.
        //let (dx, dy) = dxy(blocked.end_pos, add_pos(pos, delta_pos));
        //let next_move_result = check_collision(blocked.end_pos, dx, dy, data);
        //let new_pos = next_move_result.move_pos;

        let new_pos = blocked.end_pos;

        movement = Some(Movement::move_to(new_pos, MoveType::JumpWall));

        let next_pos = next_pos(pos, delta_pos);
        if let Some(other_id) = data.has_blocking_entity(next_pos) {
            if can_stab(data, entity_id, other_id) {
               let attack = Attack::Stab(other_id);
               movement = Some(Movement::attack(new_pos, MoveType::JumpWall, attack));
           }
        }
    } else {
        // else move up to the wall (start_pos is just before the colliding tile)
        movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
    }

    return movement;
}

pub fn entity_move_blocked_by_entity(entity_id: EntityId, other_id: EntityId, move_pos: Pos, delta_pos: Pos, data: &GameData) -> Option<Movement> {
    let movement: Option<Movement>;

    let pos = data.entities.pos[&entity_id];
    if can_stab(data, entity_id, other_id) {
        let attack = Attack::Stab(other_id);
        movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
    } else if data.entities.blocks[&other_id] {
        let attack = Attack::Push(other_id, delta_pos);
        movement = Some(Movement::attack(add_pos(pos, delta_pos), MoveType::Move, attack));
    } else {
        movement = Some(Movement::move_to(move_pos, MoveType::Move));
    }

    return movement;
}

pub fn entity_move_blocked_by_entity_and_wall(entity_id: EntityId, other_id: EntityId, blocked: &Blocked, delta_pos: Pos, data: &GameData) -> Option<Movement> {
    let movement: Option<Movement>;

    let entity_pos = data.entities.pos[&other_id];
    let pos = data.entities.pos[&entity_id];

    let entity_dist = distance(pos, entity_pos);
    let wall_dist = distance(pos, blocked.start_pos);

    if entity_dist < wall_dist {
        let attack =
            if can_stab(data, entity_id, other_id) {
                Attack::Stab(other_id)
            } else {
                Attack::Push(other_id, delta_pos)
            };
        let move_pos = move_next_to(pos, entity_pos);
        movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
    } else if entity_dist > wall_dist {
        // wall is first
        let mut jumped_wall = false;
        if data.entities.move_mode[&entity_id] == MoveMode::Run {
            if !blocked.blocked_tile && blocked.wall_type == Wall::ShortWall {
                jumped_wall = true;
            } 
        }

        if jumped_wall {
            let attack =
                if can_stab(data, entity_id, other_id) {
                    Attack::Stab(other_id)
                } else {
                    Attack::Push(other_id, delta_pos)
                };
            let move_pos = move_next_to(pos, entity_pos);
            movement = Some(Movement::attack(move_pos, MoveType::Move, attack));
        } else {
            // can't jump the wall- just move up to it.
            movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
        }
    } else {
        // entity and wall are together
        // move up to the wall- we can't jump it or attack through it
        movement = Some(Movement::move_to(blocked.start_pos, MoveType::Move));
    }

    return movement;
}

pub fn calculate_move(action: Direction,
                      reach: Reach,
                      entity_id: EntityId,
                      data: &GameData) -> Option<Movement> {
    let mut movement: Option<Movement>;

    let pos = data.entities.pos[&entity_id];

    // get the location we would move to given the input action
    if let Some(delta_pos) = reach.move_with_reach(&action) {
        let (dx, dy) = delta_pos.to_tuple();

        // check if movement collides with a blocked location or an entity
        let move_result = check_collision(pos, dx, dy, data);

        match (move_result.blocked, move_result.entity) {
            // both blocked by wall and by entity
            (Some(blocked), Some(other_id)) => {
                movement = entity_move_blocked_by_entity_and_wall(entity_id, other_id, &blocked, delta_pos, data);
            }

            // blocked by entity only
            (None, Some(other_id)) => {
                movement = entity_move_blocked_by_entity(entity_id, other_id, move_result.move_pos, delta_pos, data);
            }

            // blocked by wall only
            (Some(blocked), None) => {
                movement = entity_move_blocked_by_wall(entity_id, delta_pos, &blocked, data);
            }

            // not blocked at all
            (None, None) => {
                movement = entity_move_not_blocked(entity_id, move_result.move_pos, delta_pos, data);
            }
        }
    } else {
        // movement is not valid given the mover's reach- reject movement by return None
        movement = None;
    }

    if let Some(moved) = movement {
        if moved.attack == None && moved.pos == pos {
            movement = None;
        }
    }

    return movement;
}

pub fn direction(value: i32) -> i32 {
    if value == 0 {
        return 0;
    } else {
        return value.signum();
    }
}

