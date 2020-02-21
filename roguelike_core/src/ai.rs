use crate::map::*;
use crate::types::*;
use crate::constants::*;
use crate::movement::*;
use crate::messaging::*;
use crate::utils::*;


pub fn ai_take_turn(monster_handle: ObjectId,
                    data: &mut GameData,
                    msg_log: &mut MsgLog) {
    let turn: Action;

    match data.objects[monster_handle].ai {
        Some(Ai::Basic) => {
            turn = basic_ai_take_turn(monster_handle, data);
        }

        None => {
            turn = Action::none();
        }
    }

    ai_apply_actions(monster_handle,
                     turn,
                     data,
                     msg_log);
}

pub fn step_towards(start_pos: Pos, target_pos: Pos) -> Pos {
    let dx = target_pos.x - start_pos.x;
    let dy = target_pos.y - start_pos.y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    return Pos::new(dx, dy);
}

pub fn ai_attack(monster_handle: ObjectId,
                 target_handle: ObjectId,
                 data: &mut GameData) -> Action {
    let mut target_pos = data.objects[target_handle].pos();
    let monster_pos = data.objects[monster_handle].pos();

    let turn: Action;

    if !data.objects[target_handle].alive {
        // if the target is still alve
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else if let Some(hit_pos) =
        // if AI can hit their target
        ai_can_hit_target(&mut data.map, 
                          monster_pos,
                          target_pos,
                          &data.objects[monster_handle].attack.unwrap()) {
        turn = Action::Move(Movement::Attack(hit_pos, target_handle));
    } else if data.map.is_blocked_by_wall(monster_pos, target_pos.x - monster_pos.x, target_pos.y - monster_pos.y).is_some() {
        turn = Action::StateChange(Behavior::Investigating(target_pos));
    } else { // otherwise attempt to move towards their target
        // check positions that can hit target, filter by FOV, and get the closest.
        // then move to this closest position.
        let mut pos_offset = Pos::new(0, 0);
        if let (Some(attack), Some(movement)) =
            (data.objects[monster_handle].attack, data.objects[monster_handle].movement) {
            // get all locations they can hit
            let move_positions =
                Direction::move_actions().iter()
                                         .map(|move_action| movement.move_with_reach(move_action))
                                         .filter_map(|mov| mov)
                                         .map(|pos| add_pos(pos, monster_pos))
                                         .filter(|pos| data.map.is_within_bounds(*pos))
                                         .filter(|pos| data.is_blocked_tile(*pos))
                                         .collect::<Vec<Pos>>();

            // filter locations that are blocked or out of sight
            let positions: Vec<Pos> =
                move_positions
                .iter()
                .filter(|new_pos| ai_can_hit_target(&mut data.map, **new_pos, target_pos, &attack).is_some())
                .map(|pair| *pair)
                .collect();

            // if there are any options to move to that will allow the monster to
            // attack, move to the one closest to their current position.
            if positions.len() > 0 {
                target_pos = positions.iter()
                                      .min_by_key(|pos| distance(target_pos, Pos::from(**pos)))
                                      .map(|pair| Pos::from(*pair))
                                      .unwrap();
            }

            pos_offset = ai_take_astar_step(monster_pos, target_pos, &data);
        }

        turn = Action::Move(Movement::Move(pos_offset));
    }

    return turn;
}

pub fn ai_investigate(target_pos_orig: Pos, 
                      monster_handle: ObjectId,
                      game_data: &mut GameData) -> Action {
    let player_handle = game_data.find_player().unwrap();

    let mut target_pos = target_pos_orig;
    let player_pos = game_data.objects[player_handle].pos();
    let monster_pos = game_data.objects[monster_handle].pos();

    let turn: Action;

               
    if game_data.map.is_in_fov(monster_pos, player_pos, MONSTER_VIEW_DIST) {
        // TODO this causes a turn delay between seeing the player and attacking them
        turn = Action::StateChange(Behavior::Attacking(player_handle));
    } else { // the monster can't see the player
        if let Some(sound_pos) = game_data.sound_within_earshot(monster_pos) {
            target_pos = Pos::new(sound_pos.x, sound_pos.y);
            game_data.objects[monster_handle].behavior =
                Some(Behavior::Investigating(target_pos));
        }

        if target_pos == monster_pos { 
            // if the monster reached its target then go back to being idle
            turn = Action::StateChange(Behavior::Idle);
        } else {
            // if the monster has not reached its target, move towards the target.
            let pos_offset = ai_take_astar_step(monster_pos, target_pos, &game_data);

            turn = Action::Move(Movement::Move(pos_offset));
        }
    }

    return turn;
}

fn ai_can_hit_target(map: &mut Map,
                     monster_pos: Pos,
                     target_pos: Pos,
                     reach: &Reach) -> Option<Pos> {
    let mut hit_pos = None;

    let within_fov =
        map.is_in_fov(monster_pos,
                      target_pos,
                      MONSTER_VIEW_DIST);

    if within_fov {
            // get all locations they can hit
            let positions: Vec<Pos> =
                reach.offsets()
                .iter()
                .map(|pos| Pos::new(pos.x + monster_pos.x, pos.y + monster_pos.y))
                .collect();

        // look through attack positions, in case one hits the target
        for pos in positions {
            if target_pos == pos {
                hit_pos = Some(pos);
                break;
            }
        }
    }

    return hit_pos;
}

fn ai_take_astar_step(monster_pos: Pos,
                      target_pos: Pos,
                      game_data: &GameData) -> Pos {
    let astar_iter = game_data.map.astar(monster_pos, target_pos);

    if astar_iter.len() > 1 && !game_data.is_blocked_tile(astar_iter[1]) {
        return step_towards(monster_pos, astar_iter[1]);
    } else {
        return Pos::new(0, 0);
    }
}

// NOTE this function takes a mutable GameData because FOV requires
// mutation under the hood. It does not otherwise modify the game
pub fn basic_ai_take_turn(monster_handle: ObjectId,
                          game_data: &mut GameData) -> Action {
    let player_handle = game_data.find_player().unwrap();
    let monster_pos = game_data.objects[monster_handle].pos();
    let player_pos = game_data.objects[player_handle].pos();

    if game_data.map.is_within_bounds(monster_pos) {
        match game_data.objects[monster_handle].behavior {
            Some(Behavior::Idle) => {
                let mut turn = Action::none();

                if game_data.map.is_in_fov(monster_pos, player_pos, MONSTER_VIEW_DIST) {
                    // NOTE will cause a turn between seeing the player and attacking
                    turn = Action::StateChange(Behavior::Attacking(player_handle));
                } else if let Some(sound_pos) = game_data.sound_within_earshot(monster_pos) {
                    let sound_position = Pos::new(sound_pos.x, sound_pos.y);
                    turn = Action::StateChange(Behavior::Investigating(sound_position));
                }

                return turn;
            }

            Some(Behavior::Investigating(target_pos)) => {
                return ai_investigate(target_pos, monster_handle, game_data);
            }

            Some(Behavior::Attacking(object_handle)) => {
                return ai_attack(monster_handle, object_handle, game_data);
            }

            behavior => {
                panic!("Ai behavior {:?} unexpected!", behavior);
            }
        }
    } else {
        // position outside of map- return empty turn
        return Action::none();
    }
}

pub fn ai_apply_actions(monster_handle: ObjectId,
                        turn: Action,
                        game_data: &mut GameData,
                        msg_log: &mut MsgLog) {
    match turn {
        Action::Move(movement) => {
            match movement {
                Movement::Move(pos_offset) => {
                    let pos = game_data.objects[monster_handle].pos();

                    game_data.objects[monster_handle].set_pos(add_pos(pos, pos_offset));

                    msg_log.log(Msg::Moved(monster_handle, movement, pos));
                }

                Movement::Attack(_pos, target_handle) => {
                    attack(monster_handle, target_handle, &mut game_data.objects, msg_log);
                },

                _ => panic!("Unexpected movement!"),
            }
        },

        Action::StateChange(behavior) => {
            game_data.objects.get_mut(monster_handle).unwrap().behavior = Some(behavior);

            msg_log.log(Msg::StateChange(monster_handle, behavior));
        },

        _ => {
        }
    }
}
