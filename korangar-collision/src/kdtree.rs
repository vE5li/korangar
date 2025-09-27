use std::hash::Hash;

use hashbrown::HashMap;
use korangar_container::{SecondarySimpleSlab, SimpleKey};

use crate::AABB;
use crate::aligned_plane::{AlignedPlane, Axis};

/// Cost of traversing a kd-tree node.
/// Default value in the paper: 15.0 (Section 5.2)
const COST_TRAVERSAL: f32 = 100.0;
/// Cost of intersecting a primitive (e.g. AABB).
/// Default value in the paper: 20.0 (Section 5.2)
const COST_INTERSECTION: f32 = 20.0;
/// Bonus factor applied to empty space cuts.
/// This value is used to slightly favor splits that create empty space.
/// Default value in the paper: 0.8 (Section 3.3)
const EMPTY_CUT_BONUS: f32 = 0.8;

/// Trait that a shape has to implement, so that it can be inserted into an
/// Octree.
pub trait Insertable: Copy {
    /// Tests if the objects intersects the given AABB.
    fn intersects_aabb(&self, aabb: &AABB) -> bool;
    /// Returns the bounding AABB of the object.
    fn bounding_box(&self) -> AABB;
}

/// Trait that a shape has to implement, so that it can be used to query an
/// Octree.
pub trait Query<O> {
    /// Tests if the objects intersects the given AABB.
    fn intersects_aabb(&self, aabb: &AABB) -> bool;
    /// Tests if the query object intersects with the given object.
    fn intersects_object(&self, object: &O) -> bool;
}

/// A k-dimensional tree (KD-tree) for efficient spatial partitioning and
/// querying of objects.
///
/// This implementation is based on the construction algorithm described in:
/// "On Building Fast kd-trees for Ray Tracing, and on Doing that in O(N log N)"
/// by Wald, Ingo & Havran, Vlastimil. (2006)
///
/// It also took inspiration from "Fast and efficient implementation of
/// KD-Tree for raytracer in Rust" by Florian Amsallem.
///
/// https://flomonster.fr/articles/kdtree.html
///
/// # Performance
///
/// - Construction: O(N log N), where N is the number of objects
/// - Query: O(log N) average case, O(N) worst case
///
/// The tree uses a Surface Area Heuristic (SAH) for optimal splitting.
pub struct KDTree<K, O> {
    nodes: Vec<KDTreeNode<K>>,
    objects: SecondarySimpleSlab<K, O>,
    root_boundary: AABB,
}

/// The node layout is already optimally optimized. We could recue the size by
/// storing only the plane, but then we would need to re-create the axis when
/// querying, trading memory size for runtime cost.
enum KDTreeNode<K> {
    Node {
        left: usize,
        right: usize,
        left_boundary: AABB,
        right_boundary: AABB,
    },
    Leaf {
        keys: Vec<K>,
    },
}

impl<K> KDTreeNode<K> {
    /// Adjusts the child indices of a node by adding an offset.
    ///
    /// This function is used when relocating a subtree within a larger tree
    /// structure, by ensuring that child indices remain valid after relocating
    /// the subtree.
    fn slide(&mut self, index: usize) {
        match self {
            KDTreeNode::Node { left, right, .. } => {
                *left = left.saturating_add(index);
                *right = right.saturating_add(index);
            }
            KDTreeNode::Leaf { .. } => {}
        }
    }
}

impl<K: SimpleKey + Ord + Hash, O: Insertable> KDTree<K, O> {
    /// Creates an empty KD-tree.
    pub fn empty() -> KDTree<K, O> {
        KDTree {
            nodes: vec![],
            objects: SecondarySimpleSlab::default(),
            root_boundary: AABB::uninitialized(),
        }
    }

    /// Returns the root boundary of the tree.
    pub fn root_boundary(&self) -> AABB {
        self.root_boundary
    }

    /// Constructs a new KD-tree from a slice of key-object pairs.
    ///
    /// This method is using a O(N log N) construction algorithm.
    pub fn from_objects(insertable_objects: &[(K, O)]) -> KDTree<K, O> {
        if insertable_objects.is_empty() {
            return KDTree {
                nodes: Vec::new(),
                objects: SecondarySimpleSlab::default(),
                root_boundary: AABB::uninitialized(),
            };
        }

        let mut objects = SecondarySimpleSlab::default();
        let mut events = Vec::with_capacity(insertable_objects.len() * 6);
        let mut root_boundary = AABB::uninitialized();

        for (key, object) in insertable_objects.iter().copied() {
            objects.insert(key, object);

            let bounding_box = object.bounding_box();
            root_boundary = root_boundary.merge(&bounding_box);

            events.extend(Event::create_events(key, &bounding_box));
        }

        // We have to pre-sort all events before we start the recursion. Using an
        // unstable sort would alter the structure of the generated tree.
        events.sort();

        let nodes = Self::build_recursive(insertable_objects.len(), root_boundary, events);

        KDTree {
            nodes,
            objects,
            root_boundary,
        }
    }

    /// Recursively builds the KD-tree nodes.
    ///
    /// This method implements the core of the O(N log N) algorithm:
    /// 1. Partitions the current set of events to find the best splitting
    ///    plane.
    /// 2. If the cost of splitting is higher than the cost of a leaf, creates a
    ///    leaf node.
    /// 3. Otherwise, splits the events and recursively builds left and right
    ///    subtrees.
    fn build_recursive(object_count: usize, boundary: AABB, events: Vec<Event<K>>) -> Vec<KDTreeNode<K>> {
        let PartitionResult {
            cost,
            split_index,
            left_count,
            right_count,
        } = Self::partition(object_count, &boundary, &events);

        if cost > COST_INTERSECTION * object_count as f32 {
            let mut keys: Vec<K> = events
                .iter()
                .filter(|event| event.event_type == EventType::Start && event.axis() == Axis::X)
                .map(|event| event.object_key)
                .collect();

            keys.sort();

            return vec![KDTreeNode::Leaf { keys }];
        }

        let (left_boundary, right_boundary) = boundary.split(&events[split_index].plane);
        let (left_events, right_events) = Self::classify_and_splice(events, split_index, object_count);

        let left_subtree_nodes = Self::build_recursive(left_count, left_boundary, left_events);
        let right_subtree_nodes = Self::build_recursive(right_count, right_boundary, right_events);

        Self::flatten_subtree_to_array(left_boundary, right_boundary, left_subtree_nodes, right_subtree_nodes)
    }

    /// This method assembles the final tree structure in a flat array.
    /// The tree is stored in a pre-order traversal format, where each node is
    /// followed by its left subtree, then its right subtree.
    fn flatten_subtree_to_array(
        left_boundary: AABB,
        right_boundary: AABB,
        mut left_subtree_nodes: Vec<KDTreeNode<K>>,
        mut right_subtree_nodes: Vec<KDTreeNode<K>>,
    ) -> Vec<KDTreeNode<K>> {
        let mut nodes = vec![];

        let left_child_index = 1;
        let right_child_index = left_subtree_nodes.len() + 1;

        nodes.push(KDTreeNode::Node {
            left: left_child_index,
            right: right_child_index,
            left_boundary,
            right_boundary,
        });

        // The 'slide' operation adjusts the indices of child nodes in the left subtree.
        // This is necessary because these nodes were created assuming they start at
        // index 0, but they will be placed after the current node in the final
        // array.
        left_subtree_nodes.iter_mut().for_each(|node| node.slide(left_child_index));
        nodes.extend(left_subtree_nodes);

        // The right subtree nodes need to be adjusted by a larger offset,
        // as they will be placed after both the current node and the entire left
        // subtree.
        right_subtree_nodes.iter_mut().for_each(|node| node.slide(right_child_index));
        nodes.extend(right_subtree_nodes);

        // After these operations, 'nodes' contains the entire subtree rooted at the
        // current node, with correct child indices for all nodes.
        nodes
    }

    /// Finds the best splitting plane using the Surface Area Heuristic (SAH).
    ///
    /// This method implements the O(N) partitioning algorithm described in the
    /// paper, which sweeps through the sorted events to find the
    /// lowest-cost split.
    fn partition(object_count: usize, boundary: &AABB, events: &[Event<K>]) -> PartitionResult {
        let mut best_cost = f32::INFINITY;
        let mut best_split_index = 0;

        // Initialize counters for objects on left and right sides of the split
        // This corresponds to NL and NR in section 4.3.
        let mut left_side_axis_counts = [0; 3];
        let mut right_side_axis_counts = [object_count; 3];

        let mut best_event_left = 0;
        let mut best_event_right = object_count;

        // This implements the "sweeping" process described in Section 4.3.
        // We evaluate the cost after processing end events and before
        // processing start events. This ensures we consider all possible
        // splits, including "perfect splits" mentioned in the paper.
        events.iter().enumerate().for_each(|(index, event)| {
            let axis = event.axis();

            if event.event_type == EventType::End {
                right_side_axis_counts[axis as usize] -= 1;
            }

            let cost = Self::cost_sah(
                &event.plane,
                boundary,
                left_side_axis_counts[axis as usize],
                right_side_axis_counts[axis as usize],
            );

            if cost < best_cost {
                best_cost = cost;
                best_split_index = index;
                best_event_left = left_side_axis_counts[axis as usize];
                best_event_right = right_side_axis_counts[axis as usize];
            }

            if event.event_type == EventType::Start {
                left_side_axis_counts[axis as usize] += 1;
            }
        });

        PartitionResult {
            cost: best_cost,
            split_index: best_split_index,
            left_count: best_event_left,
            right_count: best_event_right,
        }
    }

    /// Calculates the cost of a split using the Surface Area Heuristic (SAH).
    ///
    /// The cost is based on the surface areas of the child nodes and the number
    /// of objects on each side of the split. It also applies an empty space
    /// bonus for splits that create empty nodes.
    fn cost_sah(plane: &AlignedPlane, boundary: &AABB, number_left: usize, number_right: usize) -> f32 {
        if !plane.intersects_aabb(boundary) {
            return f32::INFINITY;
        }

        let surface_boundary = boundary.surface();
        let (boundary_left, boundary_right) = boundary.split(plane);

        let surface_left = boundary_left.surface();
        let surface_right = boundary_right.surface();

        // This is the equation 5 from the paper.
        let cost = COST_TRAVERSAL
            + COST_INTERSECTION
                * ((number_left as f32 * surface_left / surface_boundary) + (number_right as f32 * surface_right / surface_boundary));

        // Apply the "empty space bonus" as described in Section 3.3 of the paper.
        //
        // The paper suggests favoring splits that create empty space by reducing
        // the cost of such splits by a constant factor (typically 0.8).
        //
        // This is the equation 7 from the paper.
        if number_left == 0 || number_right == 0 {
            cost * EMPTY_CUT_BONUS
        } else {
            cost
        }
    }

    /// Classifies objects as being on the left, right, or both sides of the
    /// splitting plane, and splits the events accordingly.
    ///
    /// This method implements the "splicing" step described in the paper, which
    /// maintains the sorted order of events without requiring a full
    /// re-sort.
    fn classify_and_splice(events: Vec<Event<K>>, best_split_index: usize, object_count: usize) -> (Vec<Event<K>>, Vec<Event<K>>) {
        let mut sides = HashMap::with_capacity(object_count);
        Self::classify_left_right_both(&events, best_split_index, &mut sides);
        Self::splice_events(events, &sides)
    }

    /// Classifies objects as being on the left, right, or both sides of the
    /// splitting plane.
    ///
    /// This method is a key part of the "splicing" step in the O(N log N)
    /// algorithm. It determines which side(s) of the splitting plane each
    /// object belongs to, based on the events up to and including the best
    /// splitting plane.
    fn classify_left_right_both(events: &[Event<K>], best_split_index: usize, sides: &mut HashMap<K, Classification>) {
        let best_split_axis = events[best_split_index].axis();

        // Iterates through events up to and including the best split.
        (0..=best_split_index).for_each(|index| {
            if events[index].axis() == best_split_axis {
                match events[index].event_type {
                    // Classifies objects as 'Left' if their end event is before the split.
                    EventType::End => {
                        sides.insert(events[index].object_key, Classification::Left);
                    }
                    // Classifies objects as 'Both' if their start event is before the split but end event is after.
                    EventType::Start => {
                        sides.insert(events[index].object_key, Classification::Both);
                    }
                }
            }
        });
        // Iterates through remaining events after the best split.
        (best_split_index..events.len()).for_each(|index| {
            // Classifies objects as 'Right' if their start event is after the split.
            if events[index].axis() == best_split_axis && events[index].event_type == EventType::Start {
                sides.insert(events[index].object_key, Classification::Right);
            }
        });
    }

    /// Splits the events into left and right lists based on the classification.
    ///
    /// This method completes the "splicing" step of the O(N log N) algorithm.
    /// It divides the sorted event list into two new sorted lists for the left
    /// and right child nodes, without requiring a full re-sort, since it
    /// preserved the relative sort order.
    fn splice_events(mut events: Vec<Event<K>>, sides: &HashMap<K, Classification>) -> (Vec<Event<K>>, Vec<Event<K>>) {
        let mut left_events = Vec::with_capacity(events.len() / 2);
        let mut right_events = Vec::with_capacity(events.len() / 2);

        // Since the input events are not needed after this step,
        // we can drain/drop it to save on memory.
        for event in events.drain(..) {
            match sides[&event.object_key] {
                Classification::Left => left_events.push(event),
                Classification::Right => right_events.push(event),
                Classification::Both => {
                    right_events.push(event);
                    left_events.push(event);
                }
            }
        }

        (left_events, right_events)
    }

    /// Queries the KD-tree for objects intersecting with the given query.
    ///
    /// This method implements an efficient traversal of the tree, pruning
    /// branches that don't intersect with the query's bounding box.
    pub fn query(&self, query: &impl Query<O>, result: &mut Vec<K>) {
        if self.nodes.is_empty() {
            return;
        }

        self.query_recursive(0, query, &self.root_boundary, result);

        result.sort_unstable();
        result.dedup();

        result.retain(|key| self.objects.get(*key).is_some_and(|object| query.intersects_object(object)));
    }

    fn query_recursive(&self, node_index: usize, query: &impl Query<O>, node_boundary: &AABB, result: &mut Vec<K>) {
        if !query.intersects_aabb(node_boundary) {
            return;
        }

        let node = &self.nodes[node_index];

        match node {
            KDTreeNode::Node {
                left,
                right,
                left_boundary,
                right_boundary,
            } => {
                self.query_recursive(*left, query, left_boundary, result);
                self.query_recursive(*right, query, right_boundary, result);
            }
            KDTreeNode::Leaf { keys } => {
                result.extend(keys.iter().copied());
            }
        }
    }
}

#[derive(Copy, Clone)]
enum Classification {
    Left,
    Right,
    Both,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum EventType {
    End,
    Start,
}

struct PartitionResult {
    cost: f32,
    split_index: usize,
    left_count: usize,
    right_count: usize,
}

#[derive(Debug, Copy, Clone)]
struct Event<K> {
    plane: AlignedPlane,
    event_type: EventType,
    object_key: K,
}

impl<K: Copy> Event<K> {
    /// Creates events for a given object's bounding box.
    ///
    /// This function generates six events for each object, corresponding to the
    /// start and end points of the object's bounding box in each dimension
    /// (X, Y, Z).
    ///
    /// The event creation and ordering aligns with the description in Section
    /// 4.3 of the paper.
    ///
    /// End events are listed before Start events for each axis, as per the
    /// paper's recommendation: "For those events with same pξ, we want to
    /// have them stored such that events with the same dimension (and thus,
    /// the same actual plane) lie together. For each of these consecutive
    /// events for the same plane, we then again use the same sort order as
    /// above: End events first, then planar events, then start events."
    fn create_events(object_key: K, aabb: &AABB) -> Vec<Event<K>> {
        vec![
            Event::new(AlignedPlane::new(Axis::X, aabb.max().x), EventType::End, object_key),
            Event::new(AlignedPlane::new(Axis::X, aabb.min().x), EventType::Start, object_key),
            Event::new(AlignedPlane::new(Axis::Y, aabb.max().y), EventType::End, object_key),
            Event::new(AlignedPlane::new(Axis::Y, aabb.min().y), EventType::Start, object_key),
            Event::new(AlignedPlane::new(Axis::Z, aabb.max().z), EventType::End, object_key),
            Event::new(AlignedPlane::new(Axis::Z, aabb.min().z), EventType::Start, object_key),
        ]
    }

    fn new(plane: AlignedPlane, event_type: EventType, object_key: K) -> Self {
        Event {
            plane,
            event_type,
            object_key,
        }
    }

    fn axis(&self) -> Axis {
        self.plane.axis()
    }

    fn distance(&self) -> f32 {
        self.plane.distance()
    }
}

impl<K: Copy> Ord for Event<K> {
    /// Compares two events for ordering.
    ///
    /// This implementation follows the sorting criteria described in
    /// Section 4.3.
    ///
    /// The sorting order is as follows:
    /// 1. Primary criterion: plane position (distance)
    /// 2. Secondary criterion: axis (dimension)
    /// 3. Tertiary criterion: event type (End before Start)
    ///
    /// This sorting is crucial for the efficient O(N log N) kd-tree
    /// construction algorithm described in the paper, particularly in the
    /// "Modified Plane-Finding Algorithm" (Algorithm 5).
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.plane
            .distance()
            .total_cmp(&other.plane.distance())
            .then_with(|| self.plane.axis().cmp(&other.plane.axis()))
            .then_with(|| other.event_type.cmp(&self.event_type))
    }
}

impl<K: Copy> PartialOrd for Event<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Copy> PartialEq for Event<K> {
    fn eq(&self, other: &Self) -> bool {
        self.distance() == other.distance() && self.axis() == other.axis() && self.event_type == other.event_type
    }
}

impl<K: Copy> Eq for Event<K> {}

#[cfg(test)]
mod tests {
    use cgmath::Point3;
    use korangar_container::create_simple_key;

    use crate::{AABB, KDTree};

    create_simple_key!(TestKey);

    #[test]
    fn test_kdtree_query() {
        let objects = vec![
            (TestKey(1), AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0))),
            (TestKey(2), AABB::new(Point3::new(2.0, 2.0, 2.0), Point3::new(3.0, 3.0, 3.0))),
            (TestKey(3), AABB::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0))),
            (TestKey(4), AABB::new(Point3::new(4.0, 4.0, 4.0), Point3::new(5.0, 5.0, 5.0))),
        ];

        let kdtree = KDTree::from_objects(&objects);

        let query_1 = AABB::new(Point3::new(0.5, 0.5, 0.5), Point3::new(0.75, 0.75, 0.75));
        let mut result_1 = Vec::new();
        kdtree.query(&query_1, &mut result_1);
        assert_eq!(result_1, vec![TestKey(1)]);

        let query_2 = AABB::new(Point3::new(1.5, 1.5, 1.5), Point3::new(2.5, 2.5, 2.5));
        let mut result_2 = Vec::new();
        kdtree.query(&query_2, &mut result_2);
        assert_eq!(result_2, vec![TestKey(2), TestKey(3)]);

        let query_3 = AABB::new(Point3::new(5.5, 5.5, 5.5), Point3::new(6.5, 6.5, 6.5));
        let mut result_3 = Vec::new();
        kdtree.query(&query_3, &mut result_3);
        assert!(result_3.is_empty());

        let query_4 = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 5.0, 5.0));
        let mut result_4 = Vec::new();
        kdtree.query(&query_4, &mut result_4);
        assert_eq!(result_4, vec![TestKey(1), TestKey(2), TestKey(3), TestKey(4)]);
    }
}
