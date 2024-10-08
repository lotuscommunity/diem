// Copyright © Diem Foundation

use crate::{
    abort_unless_feature_flag_enabled,
    natives::{
        cryptography::algebra::{
            gas::HashToGasParameters, AlgebraContext, HashToStructureSuite, Structure,
            E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        },
        helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safely_pop_arg, store_element, structure_from_ty_arg,
};
use diem_types::on_chain_config::FeatureFlag;
use ark_ec::hashing::HashToCurve;
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

fn feature_flag_of_hash_to_structure(
    structure_opt: Option<Structure>,
    suite_opt: Option<HashToStructureSuite>,
) -> Option<FeatureFlag> {
    match (structure_opt, suite_opt) {
        (Some(Structure::BLS12381G1), Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo))
        | (Some(Structure::BLS12381G2), Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

macro_rules! abort_unless_hash_to_structure_enabled {
    ($context:ident, $structure_opt:expr, $suite_opt:expr) => {
        let flag_opt = feature_flag_of_hash_to_structure($structure_opt, $suite_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! suite_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        HashToStructureSuite::try_from(type_tag).ok()
    }};
}

fn hash_to_bls12381gx_cost(
    dst_len: usize,
    msg_len: usize,
    dst_shortening_base: InternalGas,
    dst_shortening_per_byte: InternalGasPerByte,
    mapping_base: InternalGasPerArg,
    mapping_per_byte: InternalGasPerArg,
) -> InternalGas {
    // DST shortening as defined in https://www.ietf.org/archive/id/draft-irtf-cfrg-hash-to-curve-16.html#name-using-dsts-longer-than-255-.
    let dst_shortening_cost = if dst_len <= 255 {
        InternalGas::zero()
    } else {
        dst_shortening_base + dst_shortening_per_byte * NumBytes::from((17 + dst_len) as u64)
    };

    // Mapping cost. The gas formula is simplified by assuming the DST length is fixed at 256.
    let mapping_cost =
        mapping_base * NumArgs::one() + mapping_per_byte * NumArgs::from(msg_len as u64);

    dst_shortening_cost + mapping_cost
}

pub fn hash_to_internal(
    gas_params: &HashToGasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let suite_opt = suite_from_ty_arg!(context, &ty_args[1]);
    abort_unless_hash_to_structure_enabled!(context, structure_opt, suite_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let msg = bytes_ref.as_slice();
    let tag_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = tag_ref.as_bytes_ref();
    let dst = bytes_ref.as_slice();
    match (structure_opt, suite_opt) {
        (Some(Structure::BLS12381G1), Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)) => {
            context.charge(hash_to_bls12381gx_cost(
                dst.len(),
                msg.len(),
                gas_params.sha2.base,
                gas_params.sha2.per_byte,
                gas_params.algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base,
                gas_params
                    .algebra
                    .ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte,
            ))?;
            let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
                ark_ec::models::short_weierstrass::Projective<ark_bls12_381::g1::Config>,
                ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
                ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g1::Config>,
            >::new(dst)
            .unwrap();
            let new_element = <ark_bls12_381::G1Projective>::from(mapper.hash(msg).unwrap());
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        (Some(Structure::BLS12381G2), Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)) => {
            context.charge(hash_to_bls12381gx_cost(
                dst.len(),
                msg.len(),
                gas_params.sha2.base,
                gas_params.sha2.per_byte,
                gas_params.algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base,
                gas_params
                    .algebra
                    .ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte,
            ))?;
            let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
                ark_ec::models::short_weierstrass::Projective<ark_bls12_381::g2::Config>,
                ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
                ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g2::Config>,
            >::new(dst)
            .unwrap();
            let new_element = <ark_bls12_381::G2Projective>::from(mapper.hash(msg).unwrap());
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
