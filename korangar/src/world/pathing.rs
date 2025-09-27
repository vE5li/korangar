//! Implements pathfinding algorithms.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use cgmath::Vector2;
use hashbrown::{HashMap, HashSet};

const MOVE_DIAGONAL_COST: usize = 14;
const MOVE_ORTHOGONAL_COST: usize = 10;
/// The maximum size a walkable path can have.
pub const MAX_WALK_PATH_SIZE: usize = 32;

/// Essential trait that is needed to be implements for pathfinding.
pub trait Traversable {
    /// Must return `true` if the position can be walked on.
    fn is_walkable(&self, position: Vector2<usize>) -> bool;

    /// Must return `true` if the position can be shot through.
    fn is_snipeable(&self, position: Vector2<usize>) -> bool;
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct PathNode {
    position: Vector2<usize>,
    f_score: usize,
    g_score: usize,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinding algorithm for entity map navigation.
#[derive(Default)]
pub struct PathFinder {
    open_set: BinaryHeap<PathNode>,
    closed_set: HashSet<Vector2<usize>>,
    came_from: HashMap<Vector2<usize>, Vector2<usize>>,
    g_scores: HashMap<Vector2<usize>, usize>,
    path: Vec<Vector2<usize>>,
    neighbors: Vec<Vector2<usize>>,
}

impl PathFinder {
    /// Returns the shortest walkable path between start and goal. Uses a simple
    /// A* search algorithm like the legacy client and alternative server
    /// implementations. It must have the same behavior, or else we would
    /// "desync" with our client movement prediction.
    pub fn find_walkable_path(&mut self, map: &impl Traversable, start: Vector2<usize>, goal: Vector2<usize>) -> Option<&[Vector2<usize>]> {
        self.open_set.clear();
        self.closed_set.clear();
        self.came_from.clear();
        self.g_scores.clear();
        self.path.clear();

        self.open_set.push(PathNode {
            position: start,
            g_score: 0,
            f_score: Self::heuristic(start, goal),
        });
        self.g_scores.insert(start, 0);

        while let Some(current) = self.open_set.pop() {
            if current.position == goal {
                return match self.reconstruct_path(start, goal) {
                    true => Some(&self.path),
                    false => None,
                };
            }

            if self.closed_set.contains(&current.position) {
                continue;
            }
            self.closed_set.insert(current.position);

            self.find_neighbors(map, current.position);

            for neighbor in self.neighbors.drain(..) {
                if self.closed_set.contains(&neighbor) {
                    continue;
                }

                let movement_cost = if neighbor.x != current.position.x && neighbor.y != current.position.y {
                    MOVE_DIAGONAL_COST
                } else {
                    MOVE_ORTHOGONAL_COST
                };

                let tentative_g_score = current.g_score + movement_cost;

                if tentative_g_score < self.g_scores.get(&neighbor).copied().unwrap_or(usize::MAX) {
                    self.came_from.insert(neighbor, current.position);
                    self.g_scores.insert(neighbor, tentative_g_score);

                    let h_score = Self::heuristic(neighbor, goal);
                    let f_score = tentative_g_score + h_score;

                    self.open_set.push(PathNode {
                        position: neighbor,
                        g_score: tentative_g_score,
                        f_score,
                    });
                }
            }
        }

        None
    }

    /// Returns the shortest path between start and goal that can be shot
    /// through.
    // TODO: Unused for now.
    #[allow(dead_code)]
    pub fn find_snipable_path(&mut self, map: &impl Traversable, start: Vector2<usize>, goal: Vector2<usize>) -> Option<&[Vector2<usize>]> {
        self.path.clear();

        let mut current_x = start.x as isize;
        let mut current_y = start.y as isize;
        let mut target_x = goal.x as isize;
        let mut target_y = goal.y as isize;

        let mut delta_x = target_x - current_x;
        if delta_x < 0 {
            std::mem::swap(&mut current_x, &mut target_x);
            std::mem::swap(&mut current_y, &mut target_y);
            delta_x = -delta_x;
        }
        let delta_y = target_y - current_y;

        self.path.push(Vector2::new(current_x as usize, current_y as usize));

        let weight = if delta_x > delta_y.abs() { delta_x } else { delta_y.abs() };

        let mut weight_x = 0;
        let mut weight_y = 0;

        while current_x != target_x || current_y != target_y {
            weight_x += delta_x;
            weight_y += delta_y;

            if weight_x >= weight {
                weight_x -= weight;
                current_x += 1;
            }
            if weight_y >= weight {
                weight_y -= weight;
                current_y += 1;
            } else if weight_y < 0 {
                weight_y += weight;
                current_y -= 1;
            }

            if self.path.len() < MAX_WALK_PATH_SIZE {
                self.path.push(Vector2::new(current_x as usize, current_y as usize));
            } else {
                return None;
            }

            if (current_x != target_x || current_y != target_y) && !map.is_snipeable(Vector2::new(current_x as usize, current_y as usize)) {
                return None;
            }
        }

        Some(&self.path)
    }

    fn heuristic(start: Vector2<usize>, goal: Vector2<usize>) -> usize {
        let dx = (start.x as isize - goal.x as isize).unsigned_abs();
        let dy = (start.y as isize - goal.y as isize).unsigned_abs();
        let manhattan_distance = dx + dy;
        MOVE_ORTHOGONAL_COST * manhattan_distance
    }

    fn find_neighbors(&mut self, map: &impl Traversable, position: Vector2<usize>) {
        let orthogonal_neighbors = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        for (dx, dy) in orthogonal_neighbors {
            let new_x = position.x.wrapping_add_signed(dx);
            let new_y = position.y.wrapping_add_signed(dy);
            let new_position = Vector2::new(new_x, new_y);

            if map.is_walkable(new_position) {
                self.neighbors.push(new_position);
            }
        }

        let diagonal_neighbors = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

        for (dx, dy) in diagonal_neighbors {
            let new_x = position.x.wrapping_add_signed(dx);
            let new_y = position.y.wrapping_add_signed(dy);
            let new_position = Vector2::new(new_x, new_y);

            // Only allow diagonal neighbors when both adjacent orthogonal neighbors are
            // also walkable.
            if map.is_walkable(new_position)
                && map.is_walkable(Vector2::new(position.x, new_y))
                && map.is_walkable(Vector2::new(new_x, position.y))
            {
                self.neighbors.push(new_position);
            }
        }
    }

    fn reconstruct_path(&mut self, start: Vector2<usize>, goal: Vector2<usize>) -> bool {
        let mut current = goal;

        while current != start {
            self.path.push(current);
            current = *self.came_from.get(&current).unwrap();

            if self.path.len() >= MAX_WALK_PATH_SIZE {
                return false;
            }
        }

        self.path.push(start);
        self.path.reverse();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMap {
        width: usize,
        height: usize,
        not_walkable: HashSet<Vector2<usize>>,
        not_snipable: HashSet<Vector2<usize>>,
    }

    impl TestMap {
        fn new(width: usize, height: usize) -> Self {
            Self {
                width,
                height,
                not_walkable: HashSet::new(),
                not_snipable: HashSet::new(),
            }
        }

        fn set_unwalkable(&mut self, points: &[Vector2<usize>]) {
            for point in points {
                self.not_walkable.insert(*point);
            }
        }

        fn set_unsnipable(&mut self, points: &[Vector2<usize>]) {
            for point in points {
                self.not_snipable.insert(*point);
            }
        }
    }

    impl Traversable for TestMap {
        fn is_walkable(&self, position: Vector2<usize>) -> bool {
            position.x < self.width && position.y < self.height && !self.not_walkable.contains(&position)
        }

        fn is_snipeable(&self, position: Vector2<usize>) -> bool {
            position.x < self.width && position.y < self.height && !self.not_snipable.contains(&position)
        }
    }

    #[test]
    fn test_straight_path() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = Vector2::new(0, 0);
        let goal = Vector2::new(3, 0);

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();
        assert_eq!(path, vec![
            Vector2::new(0, 0),
            Vector2::new(1, 0),
            Vector2::new(2, 0),
            Vector2::new(3, 0),
        ]);
    }

    #[test]
    fn test_diagonal_path() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = Vector2::new(0, 0);
        let goal = Vector2::new(3, 3);

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();
        assert_eq!(path, vec![
            Vector2::new(0, 0),
            Vector2::new(1, 1),
            Vector2::new(2, 2),
            Vector2::new(3, 3),
        ]);
    }

    #[test]
    fn test_path_with_obstacle() {
        let mut map = TestMap::new(5, 5);
        map.set_unwalkable(&[Vector2::new(1, 1), Vector2::new(1, 2), Vector2::new(1, 3)]);

        let mut pathfinder = PathFinder::default();
        let start = Vector2::new(0, 0);
        let goal = Vector2::new(2, 2);

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();

        assert_eq!(path, vec![
            Vector2::new(0, 0),
            Vector2::new(1, 0),
            Vector2::new(2, 0),
            Vector2::new(2, 1),
            Vector2::new(2, 2),
        ]);
    }

    #[test]
    fn test_no_path_possible() {
        let mut map = TestMap::new(5, 5);

        map.set_unwalkable(&[
            Vector2::new(1, 0),
            Vector2::new(1, 1),
            Vector2::new(1, 2),
            Vector2::new(1, 3),
            Vector2::new(1, 4),
        ]);

        let mut pathfinder = PathFinder::default();

        let start = Vector2::new(0, 2);
        let goal = Vector2::new(2, 2);

        assert!(pathfinder.find_walkable_path(&map, start, goal).is_none());
    }

    #[test]
    fn test_shoot_path_straight() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = Vector2::new(0, 0);
        let goal = Vector2::new(3, 0);

        let path = pathfinder.find_snipable_path(&map, start, goal).unwrap();
        assert_eq!(path.len(), 4);

        for (index, step) in path.iter().enumerate() {
            assert_eq!(step.x, index);
            assert_eq!(step.y, 0);
        }
    }

    #[test]
    fn test_shoot_path_diagonal() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = Vector2::new(0, 0);
        let goal = Vector2::new(3, 3);

        let path = pathfinder.find_snipable_path(&map, start, goal).unwrap();
        assert_eq!(path.len(), 4);

        for (index, step) in path.iter().enumerate() {
            assert_eq!(step.x, index);
            assert_eq!(step.y, index);
        }
    }

    #[test]
    fn test_shoot_path_blocked() {
        let mut map = TestMap::new(5, 5);
        map.set_unsnipable(&[Vector2::new(1, 1)]);

        let mut pathfinder = PathFinder::default();
        let start = Vector2::new(0, 0);
        let goal = Vector2::new(2, 2);

        assert!(pathfinder.find_snipable_path(&map, start, goal).is_none());
    }
}
