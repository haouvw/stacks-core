use proptest::collection::btree_map;
use proptest::prelude::*;
use stacks_common::proptesting::*;

use crate::vm::contracts::Contract;
use crate::vm::{types::PrincipalData, ClarityVersion, ContractContext, Value};
use crate::types::{StacksHashMap as HashMap, StacksHashSet as HashSet};

use super::*;

pub fn contract_context(clarity_version: ClarityVersion) -> impl Strategy<Value = ContractContext> {
    (
        // contract_identifier
        principal_contract().prop_map(|p| match p {
            Value::Principal(PrincipalData::Contract(qual)) => qual,
            _ => unreachable!(),
        }),
        // variables
        prop::collection::vec(
            (clarity_name(), PropValue::any().prop_map_into()), 
            0..8
            ).prop_map(|v| 
                HashMap(v.into_iter().collect()
            )
        ),
        // functions
        stacks_hash_map(
            clarity_name(), 
            defined_function(), 1..5
        ),
        // defined_traits
        stacks_hash_map(
            clarity_name(), 
            btree_map(
                clarity_name(), 
                function_signature(), 
            1..5), 
            1..5
        ),
        // implemented_traits
        stacks_hash_set(trait_identifier(), 0..3),
        // persisted_names
        stacks_hash_set(clarity_name(), 0..5),
        // meta_data_map
        stacks_hash_map(
            clarity_name(),
            data_map_metadata(),
            1..5
        ),
        // meta_data_var
        stacks_hash_map(
            clarity_name(),
            data_variable_metadata(),
            1..5
        ),
        // meta_nft
        stacks_hash_map(
            clarity_name(),
            nft_metadata(),
            1..5
        ),
        // meta_ft
        stacks_hash_map(
            clarity_name(),
            ft_metadata(),
            1..5
        ),
        // data_size
        0u64..64,
    )
    .prop_map(
        move |(
            contract_identifier,
            variables,
            functions,
            defined_traits,
            implemented_traits,
            persisted_names,
            meta_data_map,
            meta_data_var,
            meta_nft,
            meta_ft,
            data_size,
        )| {
            let mut cc = ContractContext::new(contract_identifier, clarity_version);
            cc.variables = variables;
            cc.functions = functions;
            cc.defined_traits = defined_traits;
            cc.implemented_traits = implemented_traits;
            cc.persisted_names = persisted_names;
            cc.meta_data_map = meta_data_map;
            cc.meta_data_var = meta_data_var;
            cc.meta_nft = meta_nft;
            cc.meta_ft = meta_ft;
            cc.data_size = data_size;
            cc
        },
    )
}

pub fn contract() -> impl Strategy<Value = Contract> {
    clarity_version()
        .prop_flat_map(contract_context)
        .prop_map(|contract_context| 
            Contract { contract_context })
}