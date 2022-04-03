use crate::{
    position::{ContinuousDirection, Direction},
    tile::DirectionBools,
};

#[derive(Clone, Copy)]
pub struct Effects {
    pub rotate: ContinuousDirection,
    pub flip_x: bool,
    pub translate: Option<(f64, f64)>,
    pub scale: f64,
    /// color wheel radians
    pub hue_shift: f64,
    ///
    pub only_show_sides: Option<DirectionBools>,
}

impl std::fmt::Display for Effects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "transform: ")?;
        if !self.rotate.is_none() {
            write!(f, "rotate({}deg)", self.rotate.get_rotation())?;
        }
        if self.scale != 1.0 {
            let trans = (self.scale - 1.0) * 32.0;
            write!(
                f,
                "translate({}px, {}px) scale({})",
                if self.flip_x { -1.0 } else { 1.0 } * trans,
                trans,
                self.scale
            )?;
        }
        if self.flip_x {
            write!(f, "scaleX(-1)")?;
        }
        if let Some((x, y)) = self.translate {
            write!(
                f,
                "translate({}px,{}px)",
                if self.flip_x { -x } else { x },
                y
            )?;
        }
        write!(f, ";")?;
        if self.hue_shift != 0.0 {
            write!(f, "filter: hue-rotate({});", self.hue_shift)?;
        }
        if let Some(sides) = self.only_show_sides {
            for (dir, is_shown) in sides.to_items() {
                if is_shown {
                    // todo
                }
            }
        }
        Ok(())
    }
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            rotate: Direction::Up.to_continuous(),
            flip_x: false,
            translate: None,
            scale: 1.0,
            hue_shift: 0.0,
            only_show_sides: None,
        }
    }
}

#[cfg(feature = "client")]
impl Effects {
    pub fn random_rotate_flip() -> Self {
        let rotate_rand = js_sys::Math::random();
        let flip_rand = js_sys::Math::random();
        Self {
            rotate: if rotate_rand < 0.25 {
                Direction::Up
            } else if rotate_rand < 0.5 {
                Direction::Right
            } else if rotate_rand < 0.75 {
                Direction::Down
            } else {
                Direction::Left
            }
            .to_continuous(),
            flip_x: flip_rand >= 0.5,
            ..Self::default()
        }
    }
}
