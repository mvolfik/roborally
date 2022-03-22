use std::collections::{HashMap, HashSet};

use crate::{
    tile::{BeltEnd, Direction, Tile, TileGrid, TileType, WallsDescription},
    GameMap, Position,
};
use std::str::FromStr;

fn checked_split_in_two<'a, T: std::str::pattern::Pattern<'a>>(
    s: &'a str,
    delimiter: T,
) -> Option<(&'a str, &'a str)> {
    let mut split = s.split(delimiter);
    if let (Some(a), Some(b), None) = (split.next(), split.next(), split.next()) {
        Some((a, b))
    } else {
        None
    }
}

fn format_parse_error(name: &str, message: &str, value: &str) -> ParseError {
    ParseError(format!("Error parsing {}: {}: `{}`", name, message, value))
}

pub struct ParseError(String);

impl AsRef<str> for ParseError {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ParseError {
    #[inline]
    /// Gets the inner string, moving the value
    pub(crate) const fn get(self) -> String {
        self.0
    }
}

pub trait Parse: Sized {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError>;
}

trait SupportedNum: FromStr {}
impl SupportedNum for u32 {}
impl SupportedNum for u8 {}

impl<T: SupportedNum> Parse for T {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        T::from_str(value).map_err(|_| format_parse_error(name, "not a number", value))
    }
}

impl Parse for Position {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        let (x_str, y_str) = checked_split_in_two(value, ',')
            .ok_or_else(|| format_parse_error(name, "expected format `x,y`", value))?;
        Ok(Self {
            x: u32::parse(x_str, &format!("{}.x", name))?,
            y: u32::parse(y_str, &format!("{}.x", name))?,
        })
    }
}

impl Parse for Direction {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        use Direction::*;
        Ok(match value {
            "u" => Up,
            "r" => Right,
            "d" => Down,
            "l" => Left,
            _ => {
                return Err(format_parse_error(
                    name,
                    "invalid value for direction",
                    value,
                ))
            }
        })
    }
}

impl Parse for (Position, Direction) {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        let (pos, dir) = checked_split_in_two(value, ':').ok_or_else(|| {
            format_parse_error(name, "expected format `{{position}}:{{direction}}`", value)
        })?;
        Ok((
            Position::parse(pos, &format!("{}.position", name))?,
            Direction::parse(dir, &format!("{}.direction", name))?,
        ))
    }
}

impl Parse for BeltEnd {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        use BeltEnd::*;
        Ok(match value {
            "l" => TurnLeft,
            "r" => TurnRight,
            "" => Straight,
            _ => {
                return Err(format_parse_error(
                    name,
                    "invalid value for belt end",
                    value,
                ))
            }
        })
    }
}

impl<T: Parse> Parse for Vec<T> {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        value
            .split(';')
            .enumerate()
            .map(|(i, item)| T::parse(item, &format!("{}[{}]", name, i)))
            .collect()
    }
}

impl Parse for [bool; 5] {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        let mut res = [false; 5];
        let mut last_digit = 0;
        for c in value.chars() {
            match c.to_digit(10) {
                Some(d) if d > last_digit && d <= 5 => {
                    *res.get_mut(d as usize - 1).unwrap() = true;
                    last_digit = d;
                }
                _ => {
                    return Err(format_parse_error(
                        name,
                        "value isn't increasing sequence of digits in range 1..=5",
                        value,
                    ))
                }
            }
        }
        Ok(res)
    }
}

fn char_option_to_string(c_opt: Option<char>) -> String {
    match c_opt {
        Some(c) => c.to_string(),
        None => String::new(),
    }
}

impl Parse for TileType {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        use TileType::*;
        let mut chars = value.chars();
        let res = match chars.next() {
            Some('V') => Void,
            Some('F') => Floor,
            Some('B') => match chars.next() {
                None => return Err(format_parse_error(name, "missing belt type", value)),
                Some(c @ ('f' | 's')) => Belt(
                    c == 'f',
                    Direction::parse(
                        &char_option_to_string(chars.next()),
                        &format!("{}.direction", name),
                    )?,
                    BeltEnd::parse(
                        &char_option_to_string(chars.next()),
                        &format!("{}.belt-end", name),
                    )?,
                ),
                Some(_) => return Err(format_parse_error(name, "invalid belt type", value)),
            },
            Some('P') => PushPanel(
                Direction::parse(
                    &char_option_to_string(chars.next()),
                    &format!("{}.direction", name),
                )?,
                <[bool; 5]>::parse(
                    &chars.by_ref().collect::<String>(),
                    &format!("{}.active-rounds", name),
                )?,
            ),
            Some('R') => match chars.by_ref().collect::<String>().as_ref() {
                "cw" => Rotation(true),
                "ccw" => Rotation(false),
                _ => {
                    return Err(format_parse_error(
                        name,
                        "invalid rotation direction",
                        value,
                    ))
                }
            },
            #[allow(clippy::cast_possible_truncation)]
            Some('L') => Lasers(
                Direction::parse(
                    &char_option_to_string(chars.next()),
                    &format!("{}.direction", name),
                )?,
                chars
                    .next()
                    .and_then(|c| {
                        if c == '0' {
                            // disallow 0 lasers
                            None
                        } else {
                            c.to_digit(10)
                        }
                    })
                    .ok_or_else(|| format_parse_error(name, "invalid laser count", value))?
                    as u8,
            ),
            _ => return Err(format_parse_error(name, "empty tile specification", value)),
        };
        if chars.next().is_some() {
            Err(format_parse_error(
                name,
                "extra characters found at end",
                value,
            ))
        } else {
            Ok(res)
        }
    }
}

impl Parse for WallsDescription {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        let mut res = Self::default();
        for c in value.chars() {
            *match c {
                'u' => &mut res.up,
                'r' => &mut res.right,
                'd' => &mut res.down,
                'l' => &mut res.left,
                _ => {
                    return Err(format_parse_error(
                        name,
                        "invalid walls specification",
                        value,
                    ))
                }
            } = true;
        }
        Ok(res)
    }
}

impl Parse for Tile {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        let mut split = value.split(':');
        let typ = TileType::parse(split.next().unwrap(), name)?;
        let walls = if let Some(wallspec) = split.next() {
            WallsDescription::parse(wallspec, &format!("{}.walls", name))?
        } else {
            WallsDescription::default()
        };

        if split.next().is_some() {
            Err(format_parse_error(
                name,
                "expected tile specification with optinal `:{wallspec}` part",
                value,
            ))
        } else {
            Ok(Self { typ, walls })
        }
    }
}

fn get_parsed_prop<T: Parse>(
    props: &mut HashMap<&str, &str>,
    basename: &str,
    propname: &str,
    verifications: &mut [(&mut dyn FnMut(&T) -> bool, &str)],
) -> Result<T, ParseError> {
    let s = props.remove(propname).ok_or_else(|| {
        format_parse_error(
            basename,
            "missing required prop",
            &props.keys().copied().collect::<Vec<_>>().join(", "),
        )
    })?;
    let prop_fullname = &format!("{}.props.{}", basename, propname);
    let val = T::parse(s, prop_fullname)?;
    for (ver_fn, err_msg) in verifications.iter_mut() {
        if !ver_fn(&val) {
            return Err(format_parse_error(prop_fullname, err_msg, s));
        }
    }
    Ok(val)
}

impl Parse for GameMap {
    fn parse(value: &str, name: &str) -> Result<Self, ParseError> {
        let mut lines = value.lines();
        // return Err(format_parse_error("foo", "bar", lines.next().unwrap()));

        let antenna: Position;
        let reboot_token: (Position, Direction);
        let checkpoints: Vec<Position>;
        let spawn_points: Vec<(Position, Direction)>;

        let mut props = HashMap::new();
        for propdef in lines
            .next()
            .ok_or_else(|| format_parse_error(name, "no lines in input", value))?
            .split(' ')
        {
            // todo fix
            let (key, prop_value) = checked_split_in_two(propdef, '=').ok_or_else(|| {
                format_parse_error(
                    name,
                    "prop definition doesn't follow syntax `key=value`",
                    propdef,
                )
            })?;
            props.insert(key, prop_value);
        }

        let size: Position = get_parsed_prop(
            &mut props,
            name,
            "Size",
            &mut [(
                &mut |s: &Position| s.x > 0 && s.y > 0,
                "map dimensions must be non-zero",
            )],
        )?;

        let tile_lines: Vec<Vec<Tile>> = lines
            .enumerate()
            .map(|(i, line)| {
                let line_name = &format!("{}.lines[{}]", name, i);
                let line_tiles = <Vec<Tile>>::parse(line, line_name)?;
                if line_tiles.len() == size.x as usize {
                    Ok(line_tiles)
                } else {
                    Err(format_parse_error(
                        line_name,
                        "line length doesn't equal specified width",
                        line,
                    ))
                }
            })
            .collect::<Result<_, _>>()?;

        if tile_lines.len() != size.y as usize {
            return Err(format_parse_error(
                name,
                "number of tile lines doesn't equal specified height",
                &format!("<{} lines>", tile_lines.len()),
            ));
        }
        let tiles = TileGrid(tile_lines.into_iter().flatten().collect());

        {
            let mut used_special_tiles: HashSet<Position> = HashSet::new();

            let mut is_in_bounds = |p: &Position| size.contains(*p);
            let mut faces_into_map = |(pos, dir): &(Position, Direction)| {
                (pos.x > 0 || *dir != Direction::Left)
                    && (pos.y > 0 || *dir != Direction::Up)
                    && (pos.x < size.x - 1 || *dir != Direction::Right)
                    && (pos.y < size.y - 1 || *dir != Direction::Down)
            };
            let mut is_on_floor = |p: &Position| {
                tiles
                    .get_tile(size.x as usize, size.y as usize, p.x as usize, p.y as usize)
                    .map(|x| x.typ)
                    == Some(TileType::Floor)
            };
            let mut doesnt_overlap_other_special = |p: &Position| used_special_tiles.insert(*p);

            antenna = get_parsed_prop(
                &mut props,
                name,
                "Antenna",
                &mut [
                    (&mut is_in_bounds, "must be in map bounds"),
                    (&mut is_on_floor, "must be placed on a floor tile"),
                    (
                        &mut |p| {
                            matches!(
                                tiles.get_tile(
                                    size.x as usize,
                                    size.y as usize,
                                    p.x as usize,
                                    p.y as usize
                                ),
                                Some(Tile {
                                    walls: WallsDescription {
                                        up: true,
                                        right: true,
                                        down: true,
                                        left: true
                                    },
                                    ..
                                })
                            )
                        },
                        "underlying tile must have walls on all sides",
                    ),
                    (
                        &mut doesnt_overlap_other_special,
                        "can't overlap other special tiles",
                    ),
                ],
            )?;

            reboot_token = get_parsed_prop(
                &mut props,
                name,
                "Reboot",
                &mut [
                    (&mut |(pos, _)| is_in_bounds(pos), "must be in map bounds"),
                    (&mut faces_into_map, "must face into the map"),
                    (
                        &mut |(pos, _)| is_on_floor(pos),
                        "must be placed on a floor tile",
                    ),
                    (
                        &mut |(pos, _)| doesnt_overlap_other_special(pos),
                        "can't overlap other special tiles",
                    ),
                ],
            )?;

            checkpoints = get_parsed_prop(
                &mut props,
                name,
                "Checkpoints",
                &mut [
                    (
                        &mut |cps: &Vec<Position>| cps.iter().all(is_in_bounds),
                        "all must be in map bounds",
                    ),
                    (
                        &mut |cps: &Vec<Position>| cps.iter().all(is_on_floor),
                        "all must be placed on a floor tile",
                    ),
                    (
                        &mut |cps: &Vec<Position>| {
                            cps.iter().all(&mut doesnt_overlap_other_special)
                        },
                        "none can overlap other special tiles",
                    ),
                ],
            )?;

            spawn_points = get_parsed_prop(
                &mut props,
                name,
                "Spawnpoints",
                &mut [
                    (
                        &mut |sps: &Vec<(Position, Direction)>| {
                            sps.iter().all(|(pos, _)| is_in_bounds(pos))
                        },
                        "all must be in map bounds",
                    ),
                    (
                        &mut |sps: &Vec<(Position, Direction)>| sps.iter().all(faces_into_map),
                        "all must face into the map",
                    ),
                    (
                        &mut |sps: &Vec<(Position, Direction)>| {
                            sps.iter().all(|(pos, _)| is_on_floor(pos))
                        },
                        "all must be placed on a floor tile",
                    ),
                    (
                        &mut |sps: &Vec<(Position, Direction)>| {
                            sps.iter().all(|(pos, _)| doesnt_overlap_other_special(pos))
                        },
                        "none can overlap other special tiles",
                    ),
                ],
            )?;
        }
        Ok(Self {
            tiles,
            size,
            antenna,
            reboot_token,
            checkpoints,
            spawn_points,
        })
    }
}
