use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use noise::Perlin;
use noise::NoiseFn;

#[allow(unused_imports)]use tcod::console::*;
#[allow(unused_imports)]use tcod::input::{self, Event, Mouse};
#[allow(unused_imports)]use tcod::map::{Map as FovMap};

use ggez::graphics::DrawParam;
use ggez::graphics::Drawable;
use ggez::graphics::Color;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics;
use ggez::{Context, GameResult};

use mint::Point2;

use crate::engine::types::*;
use crate::engine::map::*;
use crate::input::calculate_move;
use crate::imgui_wrapper::*;
use crate::constants::*;


pub fn get_objects_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> Vec<ObjectId> {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    objects.iter()
           .enumerate()
           .filter(|(_, obj)| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
           .map(|(index, _)| index)
           .collect::<Vec<_>>()
}

pub fn rand_from_x_y(x: i32, y: i32) -> f32 {
    let mut hasher = DefaultHasher::new();

    (x as u32).hash(&mut hasher);
    (y as u32).hash(&mut hasher);
 
    let result: u64 = hasher.finish();

    return ((result & 0xFFFFFFFF) as f32) / 4294967295.0;
}

// TODO merge this with current movement overlay-
// it uses a highlight color, which is nice, and 
// checks for clear paths.
pub fn draw_movement_overlay(sprite_batch: &mut SpriteBatch,
                             map: &Map,
                             id: ObjectId,
                             config: &Config,
                             objects: &[Object]) -> Vec<(i32, i32)> {
    let mut added_positions = Vec::new();

    let color = config.color_warm_grey.color();

    if let Some(movement) = objects[id].movement {
        let offsets = movement.offsets();
        for offset in offsets {
            let x = objects[id].x as i32 + offset.0;
            let y = objects[id].y as i32 + offset.1;

            if map.clear_path((objects[id].x as i32, objects[id].y as i32), 
                              (x, y),
                              &objects) {
                draw_char(sprite_batch, '.', x, y, color);

                added_positions.push((x, y));
            }
        }
    }

    return added_positions;
}

pub fn draw_attack_overlay(sprite_batch: &mut SpriteBatch,
                           map: &Map,
                           id: ObjectId,
                           config: &Config,
                           objects: &[Object]) -> Vec<(i32, i32)> {
    let mut added_positions = Vec::new();

    let color = config.color_warm_grey.color();

    if let Some(attack) = objects[id].attack {
        let offsets = attack.offsets();
        for offset in offsets {
            let x = objects[id].x as i32 + offset.0;
            let y = objects[id].y as i32 + offset.1;

            if map.clear_path((objects[id].x as i32, objects[id].y as i32), 
                              (x, y),
                              &objects) {
                draw_char(sprite_batch, 'x', x, y, color);

                added_positions.push((x, y));
            }
        }
    }

    return added_positions;
}

pub fn lerp(first: f32, second: f32, scale: f32) -> f32 {
    return first + ((second - first) * scale);
}

pub fn lerp_color(color1: Color, color2: Color, scale: f32) -> Color {
    return Color {
        r: lerp(color1.r, color2.r, scale),
        g: lerp(color1.g, color2.g, scale),
        b: lerp(color1.b, color2.b, scale),
        a: lerp(color1.a, color2.a, scale),
    };
}

pub fn tile_color(config: &Config, x: i32, y: i32, tile: &Tile, visible: bool) -> Color {
    let perlin = Perlin::new();

    let color = match (tile.tile_type, visible) {
        (TileType::Wall, true) =>
            config.color_light_brown.color(),
        (TileType::Wall, false) =>
            config.color_dark_brown.color(),

        (TileType::Empty, true) =>
            lerp_color(config.color_tile_blue_light.color(),
                       config.color_tile_blue_dark.color(),
                       perlin.get([x as f64 / config.tile_noise_scaler,
                                   y as f64 / config.tile_noise_scaler]) as f32),

        (TileType::Empty, false) =>
            lerp_color(config.color_tile_blue_dark.color(),
                       config.color_very_dark_blue.color(),
                       perlin.get([x as f64 / config.tile_noise_scaler,
                                   y as f64 / config.tile_noise_scaler]) as f32),

        (TileType::Water, true) =>
            config.color_blueish_grey.color(),
        (TileType::Water, false) =>
            config.color_dark_brown.color(),

        (TileType::ShortWall, true) =>
            config.color_light_brown.color(),
        (TileType::ShortWall, false) =>
            config.color_dark_brown.color(),

        (TileType::Exit, true) =>
            config.color_orange.color(),
        (TileType::Exit, false) =>
            config.color_red.color(),
    };

    return color;
}

pub fn render_map(_ctx: &mut Context,
                  game: &mut Game,
                  map: &mut Map,
                  sprite_batch: &mut SpriteBatch,
                  config: &Config) {
    for y in 0..map.height() {
        for x in 0..map.width() {
            let chr;

            // Render game stuff
            let visible = game.fov.is_in_fov(x, y);

            let tile = &map.tiles[x as usize][y as usize];
            let color = tile_color(config, x, y, tile, visible);

            let explored = map.tiles[x as usize][y as usize].explored || visible;

            match tile.chr {
                Some(character) => {
                    chr = character;
                }

                None => {
                    // TODO placeholder to check if any characters are not assigned.
                    // chr should perhaps be required, not Option
                    chr = '+';
                }
            }

            draw_char(sprite_batch, chr, x, y, color);

            map.tiles[x as usize][y as usize].explored = explored;
        }
    }

}

pub fn render_sound(console: &mut dyn Console,
                    animations: &Vec<Animation>,
                    map: &Map,
                    objects: &[Object]) {
    for y in 1..map.height() {
        for x in 0..map.width() {
            // after animations play, draw sound for a frame
            if animations.len() == 0 {
               if let Some(sound_loc) = map[(x, y)].sound {
                   if map.clear_path_obstacles(sound_loc, (x, y), objects) {
                      console.put_char(x, y, '.', BackgroundFlag::None);
                   }
               }
            }
        }
    }
}

pub fn render_objects(_ctx: &mut Context,
                      fov: &FovMap,
                      objects: &[Object],
                      sprite_batch: &mut SpriteBatch) {
    let mut to_draw: Vec<_> =
        objects.iter().filter(|o| {
            fov.is_in_fov(o.x, o.y)
        }).collect();
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

    for object in &to_draw {
        draw_char(sprite_batch, object.char, object.x, object.y, object.color);
    }
}

pub fn render_overlays(game: &mut Game, sprite_batch: &mut SpriteBatch, map: &Map, objects: &[Object], config: &Config) {
    // Draw player action overlay. Could draw arrows to indicate how to reach each location
    // TODO consider drawing a very alpha grey as a highlight
    let mut highlight_color = config.color_warm_grey.color();
    highlight_color.a = config.highlight_alpha;

    for move_action in MoveAction::move_actions().iter() {
        if let Some(movement) = calculate_move(*move_action, objects[PLAYER].movement.unwrap(), PLAYER, objects, map) {
            match movement {
                Movement::Move(x, y) => {
                    draw_char(sprite_batch, MAP_SMALL_DOT_MIDDLE as char, x, y, config.color_ice_blue.color());
                    draw_char(sprite_batch, MAP_EMPTY_CHAR as char, x, y, highlight_color);
                }

                Movement::Attack(x, y, _object_id) => {
                    draw_char(sprite_batch, MAP_STAR as char, x, y, config.color_red.color());
                    draw_char(sprite_batch, MAP_EMPTY_CHAR as char, x, y, highlight_color);
                }

                Movement::Collide(x, y) => {
                    draw_char(sprite_batch, MAP_SMALL_DOT_MIDDLE as char, x, y, config.color_ice_blue.color());
                    draw_char(sprite_batch, MAP_EMPTY_CHAR as char, x, y, highlight_color);
                }

                Movement::WallKick(x, y, _dir_x, _dir_y) => {
                    draw_char(sprite_batch, MAP_SMALL_DOT_MIDDLE as char, x, y, config.color_ice_blue.color());
                    draw_char(sprite_batch, MAP_EMPTY_CHAR as char, x, y, highlight_color);
                }

                Movement::JumpWall(x, y) => {
                    draw_char(sprite_batch, MAP_SMALL_DOT_MIDDLE as char, x, y, config.color_ice_blue.color());
                    draw_char(sprite_batch, MAP_EMPTY_CHAR as char, x, y, highlight_color);
                }
            }
        }
    }
}

pub fn render_all(ctx: &mut Context,
                  game: &mut Game,
                  objects: &[Object],
                  map: &mut Map,
                  _messages: &mut Messages,
                  imgui_wrapper: &mut Gui,
                  sprite_batch: &mut SpriteBatch,
                  config: &Config)  -> GameResult<()> {
    sprite_batch.clear();

    graphics::clear(ctx, graphics::BLACK);

    render_map(ctx,
               game,
               map,
               sprite_batch,
               config);

    /* from render_objects */
    render_objects(ctx, &game.fov, objects, sprite_batch);

    // TODO removed for ggez
    // render_sound(&mut game.console, &game.animations, map, objects);

    // Draw movement and attack overlays
    // TODO removed for ggez
    render_overlays(game, sprite_batch, map, objects, config);

    // Draw UI overlay
    // let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    // let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    // TODO removed for ggez
    // render_bar(&mut game.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_RED, DARK_RED);

    //  TODO add back in lower menu

    //  TODO add back in turns, health, names under cursor

    sprite_batch.draw(ctx, Default::default())?;

    // Render game ui
    imgui_wrapper.render(ctx, map, objects);

    graphics::present(ctx)?;

    //dbg!(start_time.elapsed().as_millis());

    Ok(())
}

pub fn draw_char(sprite_batch: &mut SpriteBatch,
             chr: char,
             x: i32,
             y: i32,
             color: Color) {
    let chr_x = (chr as i32) % 16;
    let chr_y = (chr as i32) / 16;
    let draw_params =
        DrawParam {
            src: ggez::graphics::Rect {
                x: (chr_x as f32) / 16.0,
                y: (chr_y as f32) / 16.0,
                w: 1.0 / 16.0,
                h: 1.0 / 16.0,
            },
            dest: Point2 { x: x as f32 * 16.0, y: y as f32 * 16.0} ,
            rotation: 0.0,
            scale: mint::Vector2 { x: 1.0, y: 1.0 },
            offset: Point2 { x: 1.0, y: 1.0 },
            color: color,
        };

    sprite_batch.add(draw_params);
}
