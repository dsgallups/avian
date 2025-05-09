use avian3d::{
    dynamics::solver::{schedule::SubstepSolverSet, xpbd::*},
    math::*,
    prelude::*,
};
use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    prelude::*,
};
use examples_common_3d::ExampleCommonPlugin;

fn main() {
    let mut app = App::new();

    // Add plugins and startup system
    app.add_plugins((
        DefaultPlugins,
        ExampleCommonPlugin,
        PhysicsPlugins::default(),
    ))
    .add_systems(Startup, setup);

    // Get physics substep schedule and add our custom distance constraint
    let substeps = app
        .get_schedule_mut(SubstepSchedule)
        .expect("add SubstepSchedule first");
    substeps.add_systems(
        solve_constraint::<CustomDistanceConstraint, 2>
            .in_set(SubstepSolverSet::SolveUserConstraints),
    );

    // Run the app
    app.run();
}

/// A constraint that keeps the distance between two bodies at `rest_length`.
#[derive(Component)]
struct CustomDistanceConstraint {
    entity1: Entity,
    entity2: Entity,
    rest_length: Scalar,
    lagrange: Scalar,
    compliance: Scalar,
}

impl PositionConstraint for CustomDistanceConstraint {}

impl XpbdConstraint<2> for CustomDistanceConstraint {
    fn entities(&self) -> [Entity; 2] {
        [self.entity1, self.entity2]
    }
    fn clear_lagrange_multipliers(&mut self) {
        self.lagrange = 0.0;
    }
    fn solve(&mut self, bodies: [&mut RigidBodyQueryItem; 2], dt: Scalar) {
        let [body1, body2] = bodies;

        // Local attachment points at the centers of the bodies for simplicity
        let [r1, r2] = [Vector::ZERO, Vector::ZERO];

        // Compute the positional difference
        let delta_x = body1.current_position() - body2.current_position();

        // The current separation distance
        let length = delta_x.length();

        // The value of the constraint function. When this is zero, the constraint is satisfied,
        // and the distance between the bodies is the rest length.
        let c = length - self.rest_length;

        // Avoid division by zero and unnecessary computation
        if length <= 0.0 || c == 0.0 {
            return;
        }

        // Normalized delta_x
        let n = delta_x / length;

        // Compute generalized inverse masses (method from PositionConstraint)
        let w1 = self.compute_generalized_inverse_mass(body1, r1, n);
        let w2 = self.compute_generalized_inverse_mass(body2, r2, n);

        // Compute Lagrange multiplier update, essentially the signed magnitude of the correction
        let delta_lagrange =
            self.compute_lagrange_update(self.lagrange, c, &[w1, w2], self.compliance, dt);
        self.lagrange += delta_lagrange;

        // Apply positional correction (method from PositionConstraint)
        self.apply_positional_lagrange_update(body1, body2, delta_lagrange, n, r1, r2);
    }
}

impl MapEntities for CustomDistanceConstraint {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.entity1 = entity_mapper.get_mapped(self.entity1);
        self.entity2 = entity_mapper.get_mapped(self.entity2);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_mesh = meshes.add(Cuboid::default());
    let cube_material = materials.add(Color::srgb(0.8, 0.7, 0.6));

    // Spawn a static cube and a dynamic cube that is outside of the rest length
    let static_cube = commands
        .spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(cube_material.clone()),
            RigidBody::Static,
        ))
        .id();
    let dynamic_cube = commands
        .spawn((
            Mesh3d(cube_mesh),
            MeshMaterial3d(cube_material),
            Transform::from_xyz(3.0, 3.5, 0.0),
            RigidBody::Dynamic,
            MassPropertiesBundle::from_shape(&Cuboid::from_length(1.0), 1.0),
        ))
        .id();

    // Add a distance constraint to keep the cubes at a certain distance from each other.
    // The dynamic cube should swing around the static cube like a pendulum.
    commands.spawn(CustomDistanceConstraint {
        entity1: static_cube,
        entity2: dynamic_cube,
        rest_length: 2.5,
        lagrange: 0.0,
        compliance: 0.0,
    });

    // Light
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
