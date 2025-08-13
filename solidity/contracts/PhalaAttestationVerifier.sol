// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.0;

import "./BytesUtils.sol"; // adjust path if needed

interface IAttestation {
    function verifyAndAttestOnChain(
        bytes calldata input
    ) external returns (bool success, bytes memory output);

    function verifyAndAttestWithZKProof(
        bytes calldata journal,
        bytes calldata seal
    ) external returns (bool success, bytes memory output);
}

contract PhalaAttestationVerifier {
    using BytesUtils for bytes;

    address verifierAddress = 0x76A3657F2d6c5C66733e9b69ACaDadCd0B68788b;
    IAttestation public automata;

    uint16 constant HEADER_LENGTH = 48;
    uint16 constant TD_REPORT10_LENGTH = 584;

    // Make readable in Remix
    bytes public lastReportData;
    bool public lastVerified;

    constructor() {
        automata = IAttestation(verifierAddress);
    }

    // External call-through to Automata
    function verifyAndAttestOnChain(
        bytes calldata input
    ) external returns (bool success, bytes memory output) {
        return automata.verifyAndAttestOnChain(input);
    }

    // Pure extraction helper (does not touch state)
    function extractReportData(
        bytes calldata quote
    ) external pure returns (bytes memory) {
        require(
            quote.length >= HEADER_LENGTH + TD_REPORT10_LENGTH,
            "Quote too short"
        );

        // Extract the body (TD_REPORT10)
        bytes memory rawQuoteBody = quote[HEADER_LENGTH:HEADER_LENGTH +
            TD_REPORT10_LENGTH];

        // Extract 64 bytes from offset 520
        return rawQuoteBody.substring(520, 64);
    }

    // Calls external functions via `this.`; updates state; returns (success, reportData)
    function verifyAttestationAndExtractReportData(
        bytes calldata input
    ) external returns (bool success, bytes memory reportData) {
        (bool lastSuccess, ) = this.verifyAndAttestOnChain(input);
        if (!lastSuccess) {
            lastVerified = false;
            lastReportData = "";
            return (false, "");
        }

        bytes memory data = this.extractReportData(input);
        lastVerified = true;
        lastReportData = data;
        return (true, data);
    }

    // Optional explicit getters (state vars are already public)
    function getLastReportData() external view returns (bytes memory) {
        return lastReportData;
    }

    function getLastVerified() external view returns (bool) {
        return lastVerified;
    }
}
