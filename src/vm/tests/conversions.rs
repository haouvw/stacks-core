// Copyright (C) 2013-2020 Blockstack PBC, a public benefit corporation
// Copyright (C) 2020 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

pub use vm::analysis::errors::{CheckError, CheckErrors};
use vm::execute_v2;
use vm::types::BufferLength;
use vm::types::SequenceSubtype::{BufferType, StringType};
use vm::types::StringSubtype::ASCII;
use vm::types::TypeSignature::SequenceType;
use vm::types::Value;
use vm::ClarityVersion;

#[test]
fn test_simple_buff_to_int_le() {
    // For little-endian, 0001 at the beginning should be interpreted as the least significant bits.
    let good1_test = "(buff-to-int-le 0x00010000000000000000000000000000)";
    let good1_expected = Value::Int(256);
    assert_eq!(good1_expected, execute_v2(good1_test).unwrap().unwrap());

    // For signed conversion, all ff's should be negative.
    let good2_test = "(buff-to-int-le 0xffffffffffffffffffffffffffffffff)";
    let good2_expected = Value::Int(-1);
    assert_eq!(good2_expected, execute_v2(good2_test).unwrap().unwrap());

    // For little-endian, a partially filled buffer should behave as though 00's are padded on the right.
    let good3_test = "(buff-to-int-le 0x0001)";
    let good3_expected = Value::Int(256);
    assert_eq!(good3_expected, execute_v2(good3_test).unwrap().unwrap());

    // Wrong number of arguments.
    let bad_wrong_number_test =
        "(buff-to-int-le \"not-needed\" 0xfffffffffffffffffffffffffffffffe)";
    assert_eq!(
        execute_v2(bad_wrong_number_test).unwrap_err(),
        CheckErrors::IncorrectArgumentCount(1, 2).into()
    );

    // Right number of arguments, but wrong type.
    let bad_wrong_type_test = "(buff-to-int-le \"wrong-type\")";
    assert_eq!(
        execute_v2(bad_wrong_type_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(StringType(ASCII(BufferLength(10))))).into()
    );

    // Right number of arguments but buffer is too large.
    let bad_too_large_test = "(buff-to-int-le 0x000102030405060708090a0b0c0d0e0f00)";
    assert_eq!(
        execute_v2(bad_too_large_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(BufferType(BufferLength(17)))).into()
    );
}

#[test]
fn test_simple_buff_to_uint_le() {
    // For little endian, 0001 at the beginning should be interpreted as the least significant bit.
    let good1_test = "(buff-to-uint-le 0x00010000000000000000000000000000)";
    let good1_expected = Value::UInt(256);
    assert_eq!(good1_expected, execute_v2(good1_test).unwrap().unwrap());

    // For unsigned conversion, all ff's should be max positive.
    let good2_test = "(buff-to-uint-le 0xffffffffffffffffffffffffffffffff)";
    let good2_expected = Value::UInt(u128::MAX);
    assert_eq!(good2_expected, execute_v2(good2_test).unwrap().unwrap());

    // For little-endian, a partially filled buffer should behave as though 00's are padded on the right.
    let good3_test = "(buff-to-uint-le 0x0001)";
    let good3_expected = Value::UInt(256);
    assert_eq!(good3_expected, execute_v2(good3_test).unwrap().unwrap());

    // Wrong number of arguments.
    let bad_wrong_number_test =
        "(buff-to-uint-le \"not-needed\" 0xfffffffffffffffffffffffffffffffe)";
    assert_eq!(
        execute_v2(bad_wrong_number_test).unwrap_err(),
        CheckErrors::IncorrectArgumentCount(1, 2).into()
    );

    // Right number of arguments, but wrong type.
    let bad_wrong_type_test = "(buff-to-uint-le \"wrong-type\")";
    assert_eq!(
        execute_v2(bad_wrong_type_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(StringType(ASCII(BufferLength(10))))).into()
    );

    // Right number of arguments but buffer is too large.
    let bad_too_large_test = "(buff-to-uint-le 0x000102030405060708090a0b0c0d0e0f00)";
    assert_eq!(
        execute_v2(bad_too_large_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(BufferType(BufferLength(17)))).into()
    );
}

#[test]
fn test_simple_buff_to_int_be() {
    // For big-endian, 0100 at the end should be interpreted as least significant bits.
    let good1_test = "(buff-to-uint-be 0x00000000000000000000000000000100)";
    let good1_expected = Value::UInt(256);
    assert_eq!(good1_expected, execute_v2(good1_test).unwrap().unwrap());

    // For signed conversion, all ff's should be negative.
    let good2_test = "(buff-to-int-be 0xffffffffffffffffffffffffffffffff)";
    let good2_expected = Value::Int(-1);
    assert_eq!(good2_expected, execute_v2(good2_test).unwrap().unwrap());

    // For big-endian, a partially filled buffer should behave as though 00's are padded on the left.
    let good3_test = "(buff-to-int-be 0x0100)";
    let good3_expected = Value::Int(256);
    assert_eq!(good3_expected, execute_v2(good3_test).unwrap().unwrap());

    // Wrong number of arguments.
    let bad_wrong_number_test =
        "(buff-to-int-be \"not-needed\" 0xfffffffffffffffffffffffffffffffe)";
    assert_eq!(
        execute_v2(bad_wrong_number_test).unwrap_err(),
        CheckErrors::IncorrectArgumentCount(1, 2).into()
    );

    // Right number of arguments, but wrong type.
    let bad_wrong_type_test = "(buff-to-int-be \"wrong-type\")";
    assert_eq!(
        execute_v2(bad_wrong_type_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(StringType(ASCII(BufferLength(10))))).into()
    );

    // Right number of arguments but buffer is too large.
    let bad_too_large_test = "(buff-to-int-be 0x000102030405060708090a0b0c0d0e0f00)";
    assert_eq!(
        execute_v2(bad_too_large_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(BufferType(BufferLength(17)))).into()
    );
}

#[test]
fn test_simple_buff_to_uint_be() {
    // For big-endian, 0100 at the end should be interpreted as least significant bits.
    let good1_test = "(buff-to-uint-be 0x00000000000000000000000000000100)";
    let good1_expected = Value::UInt(256);
    assert_eq!(good1_expected, execute_v2(good1_test).unwrap().unwrap());

    // For unsigned conversion, all ff's should be max positive.
    let good2_test = "(buff-to-uint-be 0xffffffffffffffffffffffffffffffff)";
    let good2_expected = Value::UInt(u128::MAX);
    assert_eq!(good2_expected, execute_v2(good2_test).unwrap().unwrap());

    // For big-endian, a partially filled buffer should behave as though 00's are padded on the left.
    let good3_test = "(buff-to-uint-be 0x0100)";
    let good3_expected = Value::UInt(256);
    assert_eq!(good3_expected, execute_v2(good3_test).unwrap().unwrap());

    // Wrong number of arguments.
    let bad_wrong_number_test =
        "(buff-to-uint-be \"not-needed\" 0xfffffffffffffffffffffffffffffffe)";
    assert_eq!(
        execute_v2(bad_wrong_number_test).unwrap_err(),
        CheckErrors::IncorrectArgumentCount(1, 2).into()
    );

    // Right number of arguments, but wrong type.
    let bad_wrong_type_test = "(buff-to-uint-be \"wrong-type\")";
    assert_eq!(
        execute_v2(bad_wrong_type_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(StringType(ASCII(BufferLength(10))))).into()
    );

    // Right number of arguments but buffer is too large.
    let bad_too_large_test = "(buff-to-uint-be 0x000102030405060708090a0b0c0d0e0f00)";
    assert_eq!(
        execute_v2(bad_too_large_test).unwrap_err(),
        CheckErrors::ExpectedBuffer16(SequenceType(BufferType(BufferLength(17)))).into()
    );
}
