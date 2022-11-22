use crate::{
    position::{ContinuousDirection, Direction},
    tile::DirectionBools,
};

#[derive(Clone, Copy)]
/// This structure helps abstract transformation specification from "serialization" into CSS
pub struct Effects {
    pub rotate: ContinuousDirection,
    pub flip_x: bool,
    pub translate: Option<(f64, f64)>,
    pub scale: f64,
    /// color wheel radians
    pub hue_shift: f64,
    /// will use CSS mask-image to only show given borders
    /// probably only useful for void, defaults to None to show tile normally
    pub only_show_sides: Option<DirectionBools>,
}

impl std::fmt::Display for Effects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "transform:")?;
        write!(f, "rotate({}deg)", self.rotate.get_rotation())?;

        if (self.scale - 1.0).abs() > 0.0001 {
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
            write!(f, "filter: hue-rotate({}rad);", self.hue_shift)?;
        }
        if let Some(sides) = self.only_show_sides {
            let mask = sides
                .to_items()
                .into_iter()
                .filter_map(|(dir, is_shown)| {
                    is_shown.then(|| {
                        format!(
                            "linear-gradient({}deg, #0000 70%, #0004 85%, #000)",
                            if self.flip_x { -1 } else { 1 }
                                * (dir.to_continuous() - self.rotate).get_rotation()
                        )
                    })
                })
                .intersperse_with(|| ",".to_owned())
                .collect::<String>();
            if mask.is_empty() {
                write!(f, "opacity:0;")?;
            } else {
                write!(
                    f,
                    "mask-image:{mask};-webkit-mask-image:{mask};",
                    mask = mask
                )?;
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
    #[must_use]
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
