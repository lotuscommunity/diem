// Copyright © Diem Foundation

// Copyright (c) Diem
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::InternalGasPerArg;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub ark_bls12_381_fr_add: InternalGasPerArg,
    pub ark_bls12_381_fr_deser: InternalGasPerArg,
    pub ark_bls12_381_fr_div: InternalGasPerArg,
    pub ark_bls12_381_fr_eq: InternalGasPerArg,
    pub ark_bls12_381_fr_from_u64: InternalGasPerArg,
    pub ark_bls12_381_fr_inv: InternalGasPerArg,
    pub ark_bls12_381_fr_mul: InternalGasPerArg,
    pub ark_bls12_381_fr_neg: InternalGasPerArg,
    pub ark_bls12_381_fr_one: InternalGasPerArg,
    pub ark_bls12_381_fr_serialize: InternalGasPerArg,
    pub ark_bls12_381_fr_square: InternalGasPerArg,
    pub ark_bls12_381_fr_sub: InternalGasPerArg,
    pub ark_bls12_381_fr_zero: InternalGasPerArg,
    pub ark_bls12_381_fq12_add: InternalGasPerArg,
    pub ark_bls12_381_fq12_clone: InternalGasPerArg,
    pub ark_bls12_381_fq12_deser: InternalGasPerArg,
    pub ark_bls12_381_fq12_div: InternalGasPerArg,
    pub ark_bls12_381_fq12_eq: InternalGasPerArg,
    pub ark_bls12_381_fq12_from_u64: InternalGasPerArg,
    pub ark_bls12_381_fq12_inv: InternalGasPerArg,
    pub ark_bls12_381_fq12_mul: InternalGasPerArg,
    pub ark_bls12_381_fq12_neg: InternalGasPerArg,
    pub ark_bls12_381_fq12_one: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_u256: InternalGasPerArg,
    pub ark_bls12_381_fq12_serialize: InternalGasPerArg,
    pub ark_bls12_381_fq12_square: InternalGasPerArg,
    pub ark_bls12_381_fq12_sub: InternalGasPerArg,
    pub ark_bls12_381_fq12_zero: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_serialize_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_serialize_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_deser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_serialize_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_serialize_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_pairing: InternalGasPerArg,
    pub ark_bls12_381_multi_pairing_base: InternalGasPerArg,
    pub ark_bls12_381_multi_pairing_per_pair: InternalGasPerArg,
    pub ark_h2c_bls12381g1_xmd_sha256_sswu_base: InternalGasPerArg,
    pub ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte: InternalGasPerArg,
    pub ark_h2c_bls12381g2_xmd_sha256_sswu_base: InternalGasPerArg,
    pub ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte: InternalGasPerArg,
}

pub struct HashToGasParameters {
    pub algebra: GasParameters,
    pub sha2: diem_move_stdlib::natives::hash::Sha2_256GasParameters,
}
