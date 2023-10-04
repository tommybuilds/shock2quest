use std::collections::{HashMap, HashSet};

use cgmath::{vec3, Angle, Deg, Quaternion, Rotation3, Vector3};
use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct VRHandModelPerHandAdjustments {
    offset: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
}

impl VRHandModelPerHandAdjustments {
    pub fn new() -> VRHandModelPerHandAdjustments {
        VRHandModelPerHandAdjustments {
            offset: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 1.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn rotate_y(self, angle: Deg<f32>) -> VRHandModelPerHandAdjustments {
        VRHandModelPerHandAdjustments {
            rotation: self.rotation * Quaternion::from_angle_y(angle),
            ..self
        }
    }

    pub fn flip_x(self) -> VRHandModelPerHandAdjustments {
        VRHandModelPerHandAdjustments {
            scale: vec3(-self.scale.x, self.scale.y, self.scale.z),
            ..self
        }
    }

    pub fn flip_z(self) -> VRHandModelPerHandAdjustments {
        VRHandModelPerHandAdjustments {
            scale: vec3(self.scale.x, self.scale.y, -self.scale.z),
            ..self
        }
    }
}

#[derive(Clone)]
struct VRHandModelAdjustments {
    left_hand: VRHandModelPerHandAdjustments,
    right_hand: VRHandModelPerHandAdjustments,
}

impl VRHandModelAdjustments {
    pub fn new(
        left_hand: VRHandModelPerHandAdjustments,
        right_hand: VRHandModelPerHandAdjustments,
    ) -> VRHandModelAdjustments {
        VRHandModelAdjustments {
            left_hand,
            right_hand,
        }
    }
}

static HAND_MODEL_POSITIONING: Lazy<HashMap<&str, VRHandModelAdjustments>> = Lazy::new(|| {
    let mut map = HashMap::new();

    let held_weapon_right = VRHandModelPerHandAdjustments::new().rotate_y(Deg(90.0));
    let held_weapon_left = held_weapon_right.clone().flip_x();
    let held_weapon = VRHandModelAdjustments::new(held_weapon_left, held_weapon_right);

    let held_item_hand = VRHandModelPerHandAdjustments::new().rotate_y(Deg(180.0));
    let held_item = VRHandModelAdjustments::new(held_item_hand.clone(), held_item_hand);

    let default = VRHandModelAdjustments::new(
        VRHandModelPerHandAdjustments::new(),
        VRHandModelPerHandAdjustments::new(),
    );

    // Hand model adjustments for VR
    // Specify overrides for particular models with how they should be oriented
    // relative ot the virtual hand
    let items = vec![
        // Weapons
        ("atek_h", held_weapon.clone()),
        ("amph_h", held_weapon.clone()),
        ("lasehand", held_weapon.clone()),
        ("empgun", held_weapon.clone()),
        ("wrench_h", default.clone()),
        ("sg_w", held_weapon.clone()),
        // World items
        ("battery", held_item.clone()),
        ("batteryb", held_item.clone()),
        ("gameboy", held_item.clone()),
        ("gamecart", held_item.clone()),
        ("nanocan", held_item.clone()),
    ];

    items.iter().for_each(|(name, adjustments)| {
        map.insert(*name, adjustments.clone());
    });

    map
});

pub fn is_allowed_hand_model(model_name: &str) -> bool {
    HAND_MODEL_POSITIONING.contains_key(model_name)
}

pub fn get_vr_hand_model_adjustments(
    model_name: &str,
    is_left_hand: bool,
) -> VRHandModelPerHandAdjustments {
    let maybe_adjustments = HAND_MODEL_POSITIONING.get(model_name);

    if maybe_adjustments.is_none() {
        return VRHandModelPerHandAdjustments::new();
    }

    let adjustments = maybe_adjustments.unwrap();

    if is_left_hand {
        adjustments.left_hand.clone()
    } else {
        adjustments.right_hand.clone()
    }
}
