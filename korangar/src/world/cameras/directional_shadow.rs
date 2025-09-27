use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Transform, Vector2, Vector3, Zero};
use korangar_collision::{AABB, lerp};

use super::Camera;
use crate::graphics::{DirectionalLightPartitionInstruction, DirectionalShadowPartition, PARTITION_COUNT, orthographic_reverse_lh};

const ORIGIN: Point3<f32> = Point3::new(0.0, 0.0, 0.0);
const LOOK_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

const CAMERA_NEAR_PLANE: f32 = 0.1;
const CAMERA_FAR_PLANE: f32 = 1000.0;

/// Safety multiplier for the planes of the partitions. Needed to get some
/// safety margin while the camera moves. This combats missing shadows.
const SAFETY_MARGIN: f32 = 1.1;

pub struct DirectionalShadowCamera {
    level_bound: AABB,
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
    near_plane: f32,
    partition: [DirectionalLightPartitionInstruction; PARTITION_COUNT],
}

impl DirectionalShadowCamera {
    pub fn new() -> Self {
        Self {
            level_bound: AABB::uninitialized(),
            camera_position: ORIGIN,
            view_direction: Vector3::zero(),
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::identity(),
            view_projection_matrix: Matrix4::identity(),
            near_plane: 0.0,
            partition: [DirectionalLightPartitionInstruction::default(); PARTITION_COUNT],
        }
    }

    /// Sets the level bounds. Used to properly bound the near plane of the
    /// directional shadow.
    pub fn set_level_bound(&mut self, level_bound: AABB) {
        self.level_bound = level_bound;
    }

    /// Computes the minimum Z value of the level AABB in light view space.
    fn compute_level_min_z(&self, light_view: &Matrix4<f32>) -> f32 {
        let min = self.level_bound.min();
        let max = self.level_bound.max();

        let corners = [
            Point3::new(min.x, min.y, min.z),
            Point3::new(max.x, min.y, min.z),
            Point3::new(min.x, max.y, min.z),
            Point3::new(max.x, max.y, min.z),
            Point3::new(min.x, min.y, max.z),
            Point3::new(max.x, min.y, max.z),
            Point3::new(min.x, max.y, max.z),
            Point3::new(max.x, max.y, max.z),
        ];

        let mut min_z = f32::MAX;
        for corner in &corners {
            let transformed = light_view.transform_point(*corner);
            min_z = min_z.min(transformed.z);
        }

        min_z
    }

    fn update_main_light_camera(
        &mut self,
        direction_to_light: Vector3<f32>,
        main_camera_view: &Matrix4<f32>,
        main_camera_proj: &Matrix4<f32>,
    ) -> Matrix4<f32> {
        let direction_to_light = direction_to_light.normalize();
        self.view_direction = -direction_to_light;

        let camera_view_inv = main_camera_view
            .inverse_transform()
            .expect("Camera view matrix should be invertible");

        // Look from the light position.
        let eye = Point3::from_vec(direction_to_light);
        let at = ORIGIN;
        let up = if direction_to_light.y.abs() > 0.99 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };

        let light_view = Matrix4::look_at_lh(eye, at, up);

        let (min, max) = Self::compute_frustum_extents(
            &camera_view_inv,
            main_camera_proj,
            &light_view,
            CAMERA_NEAR_PLANE,
            CAMERA_FAR_PLANE,
        );

        // Center the light view on the frustum extents.
        let center = (min + max) * 0.5;
        let center_transform = Matrix4::from_translation(Vector3::new(-center.x, -center.y, -min.z));
        let light_view = center_transform * light_view;
        let light_view_inverse = light_view.inverse_transform().expect("Light view matrix should be invertible");

        // Create projection matrix that covers the extents.
        let dimensions = max - min;

        // Check if there is level geometry that we need to cover.
        let level_min_z = self.compute_level_min_z(&light_view);
        self.near_plane = if level_min_z < 0.0 { level_min_z } else { 0.0 };

        let light_projection = orthographic_reverse_lh(
            -dimensions.x * 0.5,
            dimensions.x * 0.5,
            -dimensions.y * 0.5,
            dimensions.y * 0.5,
            self.near_plane,
            dimensions.z,
        );

        self.camera_position = Point3::from_vec(light_view_inverse.w.truncate());
        self.view_matrix = light_view;
        self.projection_matrix = light_projection;
        self.view_projection_matrix = light_projection * light_view;

        light_view_inverse
    }

    /// Computes the axis aligned extents of the camera frustum in light space.
    fn compute_frustum_extents(
        camera_view_inv: &Matrix4<f32>,
        camera_proj: &Matrix4<f32>,
        light_view: &Matrix4<f32>,
        near_plane: f32,
        far_plane: f32,
    ) -> (Vector3<f32>, Vector3<f32>) {
        // Extract inverse scales from projection matrix to compute frustum corners.
        let scale_x_inverse = 1.0 / camera_proj.x.x;
        let scale_y_inverse = 1.0 / camera_proj.y.y;

        // Transform from camera view space to light view space.
        let camera_view_to_light_view = light_view * camera_view_inv;

        // Compute the 8 corners of the frustum in view space.
        let mut corners = [Point3::origin(); 8];

        // Near plane corners (in view space).
        let near_x = scale_x_inverse * near_plane;
        let near_y = scale_y_inverse * near_plane;
        corners[0] = Point3::new(-near_x, near_y, near_plane);
        corners[1] = Point3::new(near_x, near_y, near_plane);
        corners[2] = Point3::new(-near_x, -near_y, near_plane);
        corners[3] = Point3::new(near_x, -near_y, near_plane);

        // Far plane corners (in view space).
        let far_x = scale_x_inverse * far_plane;
        let far_y = scale_y_inverse * far_plane;
        corners[4] = Point3::new(-far_x, far_y, far_plane);
        corners[5] = Point3::new(far_x, far_y, far_plane);
        corners[6] = Point3::new(-far_x, -far_y, far_plane);
        corners[7] = Point3::new(far_x, -far_y, far_plane);

        // Transform corners to light view space and compute AABB.
        let mut min_corner = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max_corner = Vector3::new(f32::MIN, f32::MIN, f32::MIN);

        for corner in &corners {
            let transformed = camera_view_to_light_view.transform_point(*corner);
            min_corner.x = min_corner.x.min(transformed.x);
            min_corner.y = min_corner.y.min(transformed.y);
            min_corner.z = min_corner.z.min(transformed.z);
            max_corner.x = max_corner.x.max(transformed.x);
            max_corner.y = max_corner.y.max(transformed.y);
            max_corner.z = max_corner.z.max(transformed.z);
        }

        (min_corner, max_corner)
    }

    /// Updates the shadow cameras using with the bounds provided by the SDSM
    /// algorithm.
    pub fn update_camera_sdsm(
        &mut self,
        direction_to_light: Vector3<f32>,
        main_camera_view: &Matrix4<f32>,
        main_camera_projection: &Matrix4<f32>,
        shadow_map_size: u32,
        partitions: &[DirectionalShadowPartition; PARTITION_COUNT],
    ) {
        let light_view_inverse = self.update_main_light_camera(direction_to_light, main_camera_view, main_camera_projection);

        let projection_inverse = self
            .projection_matrix
            .inverse_transform()
            .expect("Projection matrix should be invertible");

        for (partition_index, partition) in partitions.iter().enumerate() {
            let expanded_partition_extents = partition.extents * SAFETY_MARGIN;

            // Un-project the NDC min/max corners back to the light's view space.
            let ndc_min = Point3::new(
                partition.center.x - expanded_partition_extents.x,
                partition.center.y - expanded_partition_extents.y,
                partition.center.z - expanded_partition_extents.z,
            );
            let ndc_max = Point3::new(
                partition.center.x + expanded_partition_extents.x,
                partition.center.y + expanded_partition_extents.y,
                partition.center.z + expanded_partition_extents.z,
            );
            let view_min = Point3::from_homogeneous(projection_inverse * ndc_min.to_homogeneous());
            let view_max = Point3::from_homogeneous(projection_inverse * ndc_max.to_homogeneous());

            let view_space_center = Point3::from_vec((view_min.to_vec() + view_max.to_vec()) * 0.5);
            let world_space_center = light_view_inverse.transform_point(view_space_center);

            let extents = (view_max.to_vec() - view_min.to_vec()) * 0.5;

            let unstable_radius = extents.x.max(extents.y);
            let quantization_steps = 4.0;
            let radius = 2.0_f32.powf((unstable_radius.log2() * quantization_steps).ceil() / quantization_steps);

            let snapped_world_center = Self::snap_to_texel_grid(world_space_center, radius, shadow_map_size, direction_to_light);
            let snapped_view_center = self.view_matrix.transform_point(snapped_world_center);

            let projection = orthographic_reverse_lh(
                snapped_view_center.x - radius,
                snapped_view_center.x + radius,
                snapped_view_center.y - radius,
                snapped_view_center.y + radius,
                self.near_plane,
                view_min.z,
            );

            self.partition[partition_index] = DirectionalLightPartitionInstruction {
                view_projection_matrix: projection * self.view_matrix,
                projection_matrix: projection,
                view_matrix: self.view_matrix,
                interval_end: partition.interval_end,
                world_space_texel_size: (2.0 * radius) / shadow_map_size as f32,
                near_plane: self.near_plane,
                far_plane: view_min.z,
            };
        }
    }

    /// PSSM partitioning scheme: Blend between logarithmic and uniform
    /// distribution. GPU Gems 3: Parallel-Split Shadow Maps on Programmable
    /// GPUs (2008).
    fn pssm_partition_from_range(partition_index: usize, min_z: f32, max_z: f32) -> f32 {
        const BLEND_FACTOR: f32 = 0.5;
        let ratio = max_z / min_z;
        let power = partition_index as f32 / PARTITION_COUNT as f32;
        let log_split = min_z * ratio.powf(power);
        let uniform_split = min_z + (max_z - min_z) * (partition_index as f32 / PARTITION_COUNT as f32);
        lerp(uniform_split, log_split, BLEND_FACTOR)
    }

    /// Updates the shadow cameras using with the bounds provided by the PSSM
    /// algorithm.
    pub fn update_camera_pssm(
        &mut self,
        direction_to_light: Vector3<f32>,
        main_camera_view: &Matrix4<f32>,
        main_camera_projection: &Matrix4<f32>,
        shadow_map_size: u32,
    ) {
        let light_view_inverse = self.update_main_light_camera(direction_to_light, main_camera_view, main_camera_projection);

        let mut near_depth = 100.0;

        let camera_view_inverse = main_camera_view
            .inverse_transform()
            .expect("Camera view matrix should be invertible");

        for partition_index in 0..PARTITION_COUNT {
            let mut light_view = self.view_matrix;
            let far_depth = Self::pssm_partition_from_range(partition_index, 100.0, CAMERA_FAR_PLANE);

            let (min, max) =
                Self::compute_frustum_extents(&camera_view_inverse, main_camera_projection, &light_view, near_depth, far_depth);

            let center = (min + max) * 0.5;

            // Use circular bounding sphere for consistent projection size.
            let dimensions = max - min;
            let unstable_radius = (dimensions.x.powi(2) + dimensions.y.powi(2) + dimensions.z.powi(2)).sqrt() * 0.5;
            let quantization_steps = 4.0;
            let radius = 2.0_f32.powf((unstable_radius.log2() * quantization_steps).ceil() / quantization_steps);
            let far_plane = radius * 2.0;

            let world_space_center = light_view_inverse.transform_point(Point3::from_vec(center));
            let snapped_world_center = Self::snap_to_texel_grid(world_space_center, radius, shadow_map_size, direction_to_light);
            let snapped_view_center = light_view.transform_point(snapped_world_center);

            let center_transform = Matrix4::from_translation(Vector3::new(
                -snapped_view_center.x,
                -snapped_view_center.y,
                -snapped_view_center.z,
            ));
            light_view = center_transform * light_view;

            // Check if there is level geometry that we need to cover.
            let level_min_z = self.compute_level_min_z(&light_view);
            let near_plane = if level_min_z < 0.0 { level_min_z } else { 0.0 };

            let light_projection = orthographic_reverse_lh(-radius, radius, -radius, radius, near_plane, far_plane);

            self.partition[partition_index] = DirectionalLightPartitionInstruction {
                view_projection_matrix: light_projection * light_view,
                projection_matrix: light_projection,
                view_matrix: light_view,
                interval_end: far_depth,
                world_space_texel_size: (radius * 2.0) / shadow_map_size as f32,
                near_plane,
                far_plane,
            };

            near_depth = far_depth;
        }
    }

    fn snap_to_texel_grid(point: Point3<f32>, radius: f32, shadow_map_size: u32, direction_to_light: Vector3<f32>) -> Point3<f32> {
        let units_per_texel = shadow_map_size as f32 / (radius * 2.0);
        let texel_scaling_matrix = Matrix4::from_scale(units_per_texel);

        let light_target = Point3::from_vec(-direction_to_light);
        let view_matrix = Matrix4::look_at_lh(Point3::origin(), light_target, Vector3::unit_y()) * texel_scaling_matrix;

        let inverse_view_matrix = view_matrix.inverse_transform().expect("Texel grid matrix should be invertible");

        let point_in_texel_space = view_matrix.transform_point(point);
        let snapped_point = Point3::new(
            point_in_texel_space.x.floor(),
            point_in_texel_space.y.floor(),
            point_in_texel_space.z,
        );

        inverse_view_matrix.transform_point(snapped_point)
    }

    /// Creates a camera for a specific partition that can be used for rendering
    pub fn get_partition_camera(&self, partition_index: usize) -> PartitionCamera {
        let partition = &self.partition[partition_index];

        PartitionCamera {
            camera_position: self.camera_position,
            view_matrix: self.view_matrix,
            projection_matrix: partition.projection_matrix,
            view_projection_matrix: partition.view_projection_matrix,
            view_direction: self.view_direction,
        }
    }

    pub fn get_partition_instructions(&self) -> [DirectionalLightPartitionInstruction; PARTITION_COUNT] {
        self.partition
    }

    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.view_projection_matrix
    }
}

pub struct PartitionCamera {
    camera_position: Point3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
    view_direction: Vector3<f32>,
}

impl Camera for PartitionCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        unimplemented!()
    }

    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {
        unimplemented!()
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        LOOK_UP
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.view_projection_matrix
    }

    fn view_direction(&self) -> Vector3<f32> {
        self.view_direction
    }
}
