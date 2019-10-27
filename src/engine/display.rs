use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use noise::Perlin;
use noise::NoiseFn;

#[allow(unused_imports)]use tcod::map::{Map as FovMap};

use ggez::graphics::{draw, set_canvas, Canvas, Rect, Color, Drawable, DrawParam, Image, screen_coordinates};
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics;
use ggez::conf::NumSamples;
use ggez::{Context, GameResult};

use mint::Point2;

use crate::engine::types::*;
use crate::engine::map::*;
use crate::input::calculate_move;
use crate::imgui_wrapper::*;
use crate::constants::*;
use crate::plat::*;


pub struct DisplayState {
    pub imgui_wrapper: Gui,
    pub font_image: Image,
    pub background_image: Option<Image>,
    pub redraw_background: bool,
    pub map_canvas: Canvas,
    pub sprite_batch: SpriteBatch,
    pub display_overlays: bool,
    pub screen_sections: Plan,
}

impl DisplayState {
    pub fn new(font_image: Image, ctx: &mut Context) -> DisplayState {
        let imgui_wrapper = Gui::new(ctx);

        let sprite_batch = SpriteBatch::new(font_image.clone());

        let map_width_pixels  = (FONT_WIDTH * MAP_WIDTH) as u16;
        let map_height_pixels = (FONT_HEIGHT * MAP_HEIGHT) as u16;
        DisplayState {
            imgui_wrapper,
            font_image,
            redraw_background: true,
            background_image: None,
            map_canvas: Canvas::new(ctx, map_width_pixels, map_height_pixels, NumSamples::One).unwrap(),
            sprite_batch,
            display_overlays: false,
            screen_sections: Plan::empty(),
        }
    }
}


pub struct Area {
    x_offset: f32,
    y_offset: f32,
    font_width: usize,
    font_height: usize,
}

impl Area {
    pub fn new(x_offset: f32, y_offset: f32, font_width: usize, font_height: usize) -> Area {
        Area { x_offset,
               y_offset,
               font_width,
               font_height,
        }
    }
}


pub fn get_objects_under_mouse(x: i32, y: i32, objects: &[Object], fov_map: &FovMap) -> Vec<ObjectId> {
    let mut object_ids = Vec::new();

    for object_index in 0..objects.len() {
        if objects[object_index].pos() == (x, y) {
            if fov_map.is_in_fov(x, y) {
                object_ids.push(object_index);
            }
        }
    }

    return object_ids;
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
                             area: &Area,
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
                draw_char(sprite_batch, '.', x, y, color, area);

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
                           area: &Area,
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
                draw_char(sprite_batch, 'x', x, y, color, area);

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

pub fn empty_tile_color(config: &Config, x: i32, y: i32, visible: bool) -> Color {
    let perlin = Perlin::new();

    let low_color;
    let high_color;
    if visible {
        low_color = config.color_tile_blue_light.color();
        high_color = config.color_tile_blue_dark.color();
    } else {
        low_color = config.color_tile_blue_dark.color();
        high_color = config.color_very_dark_blue.color();
    }
    let color =
        lerp_color(low_color,
                   high_color,
                   perlin.get([x as f64 / config.tile_noise_scaler,
                               y as f64 / config.tile_noise_scaler]) as f32);

   return color;
}

pub fn tile_color(config: &Config, _x: i32, _y: i32, tile: &Tile, visible: bool) -> Color {
    let color = match (tile.tile_type, visible) {
        (TileType::Wall, true) =>
            config.color_light_brown.color(),
        (TileType::Wall, false) =>
            config.color_dark_brown.color(),

        (TileType::Empty, true) =>
            config.color_light_brown.color(),

        (TileType::Empty, false) =>
            config.color_dark_brown.color(),

        (TileType::Water, true) =>
            config.color_blueish_grey.color(),
        (TileType::Water, false) =>
            config.color_blueish_grey.color(),

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

pub fn render_background(fov: &FovMap,
                         map: &mut Map,
                         sprite_batch: &mut SpriteBatch,
                         area: &Area,
                         config: &Config) {
    for y in 0..map.height() {
        for x in 0..map.width() {
            let visible = fov.is_in_fov(x, y);
            draw_char(sprite_batch,
                      MAP_EMPTY_CHAR as char,
                      x,
                      y,
                      empty_tile_color(config, x, y, visible),
                      area);

            let tile = &map.tiles[x as usize][y as usize];
            if tile.tile_type == TileType::Water {
                let color = tile_color(config, x, y, tile, visible);
                let chr = tile.chr.map_or('+', |chr| chr);
                draw_char(sprite_batch, chr, x, y, color, area);
            }
        }
    }

}

pub fn render_map(fov: &FovMap,
                  map: &mut Map,
                  sprite_batch: &mut SpriteBatch,
                  area: &Area,
                  config: &Config) {
    let map_width = map.width();
    let map_height = map.height();

    for y in 0..map_height {
        for x in 0..map_width {
            // Render game stuff
            let visible = fov.is_in_fov(x, y);

            let tile = &map.tiles[x as usize][y as usize];
            let color = tile_color(config, x, y, tile, visible);

            let explored = map.tiles[x as usize][y as usize].explored || visible;

            let wall_color;
            if explored {
                wall_color = config.color_light_brown.color();
            } else {
                wall_color = config.color_dark_brown.color();
            }

            let chr = tile.chr.map_or('+', |chr| chr);

            // draw empty tile first, in case there is transparency in the character
            // draw_char(sprite_batch, MAP_EMPTY_CHAR as char, x, y, empty_tile_color(config, x, y, visible));

            // if the tile is not empty or water, draw it
            if chr != MAP_EMPTY_CHAR as char && tile.tile_type != TileType::Water {
                draw_char(sprite_batch, chr, x, y, color, area);
            }

            // finally, draw the between-tile walls appropriate to this tile
            if tile.bottom_wall == Wall::ShortWall {
                draw_char(sprite_batch, MAP_THIN_WALL_BOTTOM as char, x, y, wall_color, area);
            } else if tile.bottom_wall == Wall::TallWall {
                draw_char(sprite_batch, MAP_THICK_WALL_BOTTOM as char, x, y, wall_color, area);
            }

            if tile.left_wall == Wall::ShortWall {
                draw_char(sprite_batch, MAP_THIN_WALL_LEFT as char, x, y, wall_color, area);
            } else if tile.left_wall == Wall::TallWall {
                draw_char(sprite_batch, MAP_THICK_WALL_LEFT as char, x, y, wall_color, area);
            }

            if x + 1 < map_width {
                let right_tile = &map.tiles[x as usize + 1][y as usize];
                if right_tile.left_wall == Wall::ShortWall {
                    draw_char(sprite_batch, MAP_THIN_WALL_RIGHT as char, x, y, wall_color, area);
                } else if right_tile.left_wall == Wall::TallWall {
                    draw_char(sprite_batch, MAP_THICK_WALL_RIGHT as char, x, y, wall_color, area);
                }
            }

            if y - 1 >= 0 {
                let above_tile = &map.tiles[x as usize][y as usize - 1];
                if above_tile.bottom_wall == Wall::ShortWall {
                    draw_char(sprite_batch, MAP_THIN_WALL_TOP as char, x, y, wall_color, area);
                } else if above_tile.bottom_wall == Wall::TallWall {
                    draw_char(sprite_batch, MAP_THICK_WALL_TOP as char, x, y, wall_color, area);
                }
            }

            map.tiles[x as usize][y as usize].explored = explored;
        }
    }

}

pub fn render_objects(_ctx: &mut Context,
                      fov: &FovMap,
                      objects: &[Object],
                      area: &Area,
                      sprite_batch: &mut SpriteBatch) {
    let mut to_draw: Vec<_> =
        objects.iter().filter(|o| {
            fov.is_in_fov(o.x, o.y)
        }).collect();
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });

    for object in &to_draw {
        draw_char(sprite_batch, object.char, object.x, object.y, object.color, area);
    }
}

pub fn render_overlays(mouse_state: &MouseState,
                       fov: &FovMap,
                       sprite_batch: &mut SpriteBatch,
                       map: &Map,
                       objects: &[Object],
                       area: &Area,
                       config: &Config) {
    // Draw player action overlay. Could draw arrows to indicate how to reach each location
    // TODO consider drawing a very alpha grey as a highlight
    let mut highlight_color = config.color_warm_grey.color();
    highlight_color.a = config.highlight_alpha;

    // Draw player movement overlay
    for move_action in MoveAction::move_actions().iter() {
        // for all movements except staying still
        if *move_action != MoveAction::Center {
            // calculate the move that would occur
            if let Some(movement) = calculate_move(*move_action, objects[PLAYER].movement.unwrap(), PLAYER, objects, map) {
                // draw a highlight on that square
                let xy = movement.xy();
                draw_char(sprite_batch, MAP_EMPTY_CHAR as char, xy.0, xy.1, highlight_color, area);
            }
        }
    }

    let mut attack_highlight_color = config.color_red.color();
    attack_highlight_color.a = config.highlight_alpha;
    // Draw monster attack overlay
    let mouse_x = (mouse_state.pos.0 as i32 / FONT_WIDTH) + 1;
    let mouse_y = (mouse_state.pos.1 as i32 / FONT_HEIGHT) + 1;
    let object_ids =  get_objects_under_mouse(mouse_x, mouse_y, objects, &fov);
    for object_id in object_ids.iter() {
        if let Some(reach) = objects[*object_id].attack {
            let attack_positions = 
                reach.offsets()
                     .iter()
                     .map(|offset| (mouse_x + offset.0,
                                    mouse_y + offset.1))
                     .collect::<Vec<(i32, i32)>>();

            for position in attack_positions {
                draw_char(sprite_batch, MAP_EMPTY_CHAR as char, position.0, position.1, attack_highlight_color, area);
            }
        }
    }
}

pub fn render_all(ctx: &mut Context,
                  mouse_state: &mut MouseState,
                  objects: &[Object],
                  map: &mut Map,
                  fov: &FovMap,
                  display_state: &mut DisplayState,
                  config: &Config)  -> GameResult<()> {
    graphics::clear(ctx, graphics::BLACK);

    let screen_rect = screen_coordinates(ctx);

    let plots = display_state.screen_sections
                             .plot(0,
                                   0,
                                   screen_rect.w.abs() as usize,
                                   screen_rect.h.abs() as usize);
    for plot in plots {
        let plot_rect = Rect::new(plot.x as f32, plot.y as f32, plot.width as f32, plot.height as f32);

        //if plot.contains(mouse_state.x, mouse_state.y) {
        //    let (new_x, y_new) = plot.within(mouse_state.x, mouse_state.y);
        //    mouse_state.x_within = new_x;
        //    mouse_state.y_within = new_y;
        //    mouse_state.area_name = plot.name();
        //}

        match plot.name.as_str() {
            "screen" => {
            }

            "map" => {
                let ((x_offset, y_offset), scaler) =
                    plot.fit(map.width() as usize * FONT_WIDTH as usize, map.height() as usize * FONT_HEIGHT as usize);
                let area = Area::new(x_offset,
                                     y_offset,
                                     (scaler * FONT_WIDTH as f32) as usize,
                                     (scaler * FONT_HEIGHT as f32) as usize);

                render_background(fov,
                                  map,
                                  &mut display_state.sprite_batch,
                                  &area,
                                  config);

                render_map(fov,
                           map,
                           &mut display_state.sprite_batch,
                           &area,
                           config);

                render_objects(ctx,
                               fov,
                               objects,
                               &area,
                               &mut display_state.sprite_batch);

                render_overlays(mouse_state,
                                fov,
                                &mut display_state.sprite_batch,
                                map,
                                objects,
                                &area,
                                config);

                display_state.sprite_batch.draw(ctx, DrawParam::default()).unwrap();
                display_state.sprite_batch.clear();
            }

            "inspector" => {
                // Render game ui
                display_state.imgui_wrapper.render(ctx, map, objects, mouse_state, plot.dims(), plot.pos());
            }

            section_name => {
                panic!(format!("Unexpected screen section '{}'", section_name));
            }
        }
    }

    graphics::present(ctx)?;

    Ok(())
}

pub fn draw_char(sprite_batch: &mut SpriteBatch,
                 chr: char,
                 x: i32,
                 y: i32,
                 color: Color,
                 area: &Area) {
    let chr_x = (chr as i32) % FONT_WIDTH;
    let chr_y = (chr as i32) / FONT_HEIGHT;

    let font_part = 1.0 / FONT_WIDTH as f32;
    let pixel = font_part / FONT_HEIGHT as f32;

    let dest_x = area.font_width as f32 / (area.font_width as f32);
    let dest_y = area.font_height as f32 / (area.font_height as f32);
    let draw_params =
        DrawParam {
            src: ggez::graphics::Rect {
                x: chr_x as f32 * font_part, // + pixel,
                y: chr_y as f32 * font_part, // + pixel,
                w: font_part,// - pixel * 2.0,
                h: font_part,// - pixel * 2.0,
            },
            dest: Point2 { x: x as f32 * (area.font_width as f32),//  - 2.0),
                           y: y as f32 * (area.font_height as f32), }, // - 2.0) },
            rotation: 0.0,
            scale: mint::Vector2 { x: dest_x, // - 2.0),
                                   y: dest_y, }, // - 2.0) },
            offset: Point2 { x: 0.0, y: 0.0 },
            color: color,
        };

    sprite_batch.add(draw_params);
}
