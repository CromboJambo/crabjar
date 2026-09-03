//! Minimal cube OBJ payload used as the placeholder crab body.
//!
//! A unit cube centered at the origin. Color is applied at placement time
//! (an RGP placement attribute), not baked into the mesh, so every crab can
//! share this geometry and still render in a different color.

/// Returns the OBJ text for a unit cube centered at the origin.
#[must_use]
pub fn cube_obj() -> String {
    let mut s = String::with_capacity(512);
    s.push_str("# crabjar-terrarium cube placeholder\n");
    s.push_str("v -0.5 -0.5 0.5\n");
    s.push_str("v  0.5 -0.5 0.5\n");
    s.push_str("v  0.5  0.5 0.5\n");
    s.push_str("v -0.5  0.5 0.5\n");
    s.push_str("v -0.5 -0.5 -0.5\n");
    s.push_str("v  0.5 -0.5 -0.5\n");
    s.push_str("v  0.5  0.5 -0.5\n");
    s.push_str("v -0.5  0.5 -0.5\n");
    s.push_str("f 1 2 3 4\n");
    s.push_str("f 5 8 7 6\n");
    s.push_str("f 1 5 6 2\n");
    s.push_str("f 3 7 8 4\n");
    s.push_str("f 1 4 8 5\n");
    s.push_str("f 2 6 7 3\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_has_eight_vertices_and_six_faces() {
        let obj = cube_obj();
        let verts = obj.lines().filter(|l| l.starts_with('v')).count();
        let faces = obj.lines().filter(|l| l.starts_with('f')).count();
        assert_eq!(verts, 8);
        assert_eq!(faces, 6);
    }
}
