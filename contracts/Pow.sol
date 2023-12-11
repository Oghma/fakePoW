// SPDX-License-Identifier: MIT

pragma solidity ^0.8.20;

contract Pow {
    function mine(
        uint256 nonce1,
        uint256 nonce2
    ) public pure returns (bytes32 hashed) {
        hashed = keccak256(abi.encode(nonce1, nonce2));
    }
}
