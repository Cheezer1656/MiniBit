/*
    MiniBit - A Minecraft minigame server network written in Rust.
    Copyright (C) 2024  Cheezer1656 (https://github.com/Cheezer1656/)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::VecDeque;

use valence::math::Vec3;

pub struct Collider {
    pub vertices: Vec<Vec3>,
}

impl Collider {
    pub fn new(points: Vec<Vec3>) -> Self {
        Self { vertices: points }
    }
    
    pub fn find_farthest(&self, direction: Vec3) -> Vec3 {
        let mut max_point = Vec3::ZERO;
        let mut max_dist = -std::f32::INFINITY;

        for vertex in &self.vertices {
            let dist = vertex.dot(direction);
            if dist > max_dist {
                max_dist = dist;
                max_point = *vertex;
            }
        }

        assert_ne!(max_dist, -std::f32::INFINITY);

        max_point
    }
}

fn support(col_a: &Collider, col_b: &Collider, direction: Vec3) -> Vec3 {
    let a = col_a.find_farthest(direction);
    let b = col_b.find_farthest(-direction);

    a - b
}

struct Simplex {
    points: VecDeque<Vec3>,
}

impl Default for Simplex {
    fn default() -> Self {
        Self {
            points: VecDeque::with_capacity(4),
        }
    }
}

impl Simplex {
    pub fn new(points: Vec<Vec3>) -> Self {
        Self {
            points: points.into_iter().collect(),
        }
    }

    pub fn push_front(&mut self, point: Vec3) {
        if self.points.len() == 4 {
            self.points.pop_back();
        }
        self.points.push_front(point);
    }

    pub fn size(&self) -> usize {
        self.points.len()
    }

    pub fn get(&self, index: usize) -> Vec3 {
        self.points[index]
    }
}

pub fn gjk(col_a: &Collider, col_b: &Collider) -> bool {
    let mut sup = Vec3::ZERO;

    let mut points = Simplex::default();
    points.push_front(sup);

    let mut dir = -sup;

    loop {
        sup = support(&col_a, &col_b, dir);

        if sup.dot(dir) < 0.0 {
            return false;
        }

        points.push_front(sup);

        if next_simplex(&mut points, &mut dir) {
            return true;
        }
    }
}

fn next_simplex(points: &mut Simplex, dir: &mut Vec3) -> bool {
    match points.size() {
        2 => line_case(points, dir),
        3 => triangle_case(points, dir),
        4 => tetrahedron_case(points, dir),
        _ => unreachable!(),
    }
}

fn same_dir(a: &Vec3, b: &Vec3) -> bool {
    a.dot(*b) > 0.0
}

fn line_case(points: &mut Simplex, dir: &mut Vec3) -> bool {
    let a = points.get(0);
    let b = points.get(1);

    let ab = b - a;
    let ao = -a;

    if same_dir(&ab, &ao) {
        *dir = ab.cross(ao).cross(ab);
    } else {
        *points = Simplex::new(vec![a]);
        *dir = ao;
    }

    false
}

fn triangle_case(points: &mut Simplex, dir: &mut Vec3) -> bool {
    let a = points.get(0);
    let b = points.get(1);
    let c = points.get(2);

    let ab = b - a;
    let ac = c - a;
    let ao = -a;

    let abc = ab.cross(ac);

    if same_dir(&abc.cross(ac), &ao) {
        if same_dir(&ac, &ao) {
            *points = Simplex::new(vec![a, c]);
            *dir = ac.cross(ac.cross(ao));
        } else {
            return line_case(&mut Simplex::new(vec![a, b]), dir);
        }
    } else {
        if same_dir(&ab.cross(abc), &ao) {
            return line_case(&mut Simplex::new(vec![a, b]), dir);
        } else {
            if same_dir(&abc, &ao) {
                *dir = abc;
            } else {
                *points = Simplex::new(vec![a, c, b]);
                *dir = -abc;
            }
        }
    }

    false
}

fn tetrahedron_case(points: &mut Simplex, dir: &mut Vec3) -> bool {
    let a = points.get(0);
    let b = points.get(1);
    let c = points.get(2);
    let d = points.get(3);

    let ab = b - a;
    let ac = c - a;
    let ad = d - a;
    let ao = -a;

    let abc = ab.cross(ac);
    let acd = ac.cross(ad);
    let adb = ad.cross(ab);

    if same_dir(&abc, &ao) {
        return triangle_case(&mut Simplex::new(vec![a, b, c]), dir);
    }

    if same_dir(&acd, &ao) {
        return triangle_case(&mut Simplex::new(vec![a, c, d]), dir);
    }

    if same_dir(&adb, &ao) {
        return triangle_case(&mut Simplex::new(vec![a, d, b]), dir);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support() {
        let col_a = Collider::new(vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ]);

        let col_b = Collider::new(vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ]);

        let dir = Vec3::new(1.0, 1.0, 1.0);

        let result = support(&col_a, &col_b, dir);

        assert_eq!(result, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_gjk() {
        let col_a = Collider::new(vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ]);

        let col_b = Collider::new(vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ]);

        let result = gjk(&col_a, &col_b);

        assert_eq!(result, true);
    }
}