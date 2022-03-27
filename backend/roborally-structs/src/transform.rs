#[derive(Default, Clone, Copy)]
pub struct Transform {
    pub rotate: Option<f64>,
    pub flip_x: bool,
    pub translate: Option<(f64, f64)>,
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flip_x {
            write!(f, "scaleX(-1)")?;
        }
        if let Some(deg) = self.rotate {
            write!(f, "rotate({}deg)", if self.flip_x { -deg } else { deg })?;
        }
        if let Some((x, y)) = self.translate {
            write!(
                f,
                "translate({}px,{}px)",
                if self.flip_x { -x } else { x },
                y
            )?;
        }
        Ok(())
    }
}
