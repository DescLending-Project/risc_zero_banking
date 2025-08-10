// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "./CreditScore.sol";
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {Receipt} from "risc0/IRiscZeroVerifier.sol";

// Mock verifier that always passes (for nullifier testing only)
contract MockRiscZeroVerifier is IRiscZeroVerifier {
    function verify(bytes calldata, bytes32, bytes32) external pure override {}

    function verifyIntegrity(Receipt calldata) external pure override {}
}

contract NullifierTest is Test {
    CreditScore public creditContract;
    MockRiscZeroVerifier public mockVerifier;

    // Test nullifiers
    bytes32 constant LENDER_NULLIFIER = keccak256("lender1");
    bytes32 constant OWNED_NULLIFIER_1 = keccak256("owned1");
    bytes32 constant OWNED_NULLIFIER_2 = keccak256("owned2");
    bytes32 constant DIFFERENT_LENDER = keccak256("different_lender");

    // Test addresses
    address constant USER1 = address(0x1);
    address constant USER2 = address(0x2);

    function setUp() public {
        mockVerifier = new MockRiscZeroVerifier();
        creditContract = new CreditScore(mockVerifier);
    }

    // Helper to create minimal valid journal data (focusing only on nullifiers)
    function createJournalData(
        address userAddress,
        bytes32[] memory nullifiers,
        string memory tradifyNullifier
    ) internal view returns (CreditScore.JournalData memory) {
        return
            CreditScore.JournalData({
                score: 750,
                serverName: "openbanking-api-826260723607.europe-west3.run.app",
                stateRootProvider: "sonic-blaze.g.alchemy.com",
                blockNumber: uint64(block.number),
                tradifyNullifier: tradifyNullifier,
                tradfiDateTimestamp: uint64(block.timestamp),
                userAddress: userAddress,
                allNullifiers: nullifiers
            });
    }

    // ============= ADD NULLIFIERS TESTS =============

    function testAddNullifiers_SingleAccount() public {
        bytes32[] memory nullifiers = new bytes32[](1);
        nullifiers[0] = LENDER_NULLIFIER;

        CreditScore.JournalData memory journalData = createJournalData(
            USER1,
            nullifiers,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData, "attestation");

        // Verify nullifier is marked as used
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender nullifier should be marked as used"
        );

        // Verify tradify mapping
        assertEq(
            creditContract.tradifyNullifiers("tradify1"),
            LENDER_NULLIFIER,
            "Tradify nullifier should map to lender nullifier"
        );
    }

    function testAddNullifiers_MultipleAccounts() public {
        bytes32[] memory nullifiers = new bytes32[](3);
        nullifiers[0] = LENDER_NULLIFIER;
        nullifiers[1] = OWNED_NULLIFIER_1;
        nullifiers[2] = OWNED_NULLIFIER_2;

        CreditScore.JournalData memory journalData = createJournalData(
            USER1,
            nullifiers,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData, "attestation");

        // Verify all nullifiers are marked as used
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender nullifier should be marked as used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "First owned nullifier should be marked as used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_2),
            "Second owned nullifier should be marked as used"
        );
    }

    function testAddNullifiers_ReuseOwnLenderAccount() public {
        // First submission
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // Second submission - same user, same lender account (should succeed)
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = LENDER_NULLIFIER; // Reuse same lender
        nullifiers2[1] = OWNED_NULLIFIER_1; // Add owned account

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER1,
            nullifiers2,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData2, "attestation2");

        // Both nullifiers should be used
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender nullifier should still be used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "Owned nullifier should be marked as used"
        );
    }

    function testAddNullifiers_WrongTradifyOwner_Reverts() public {
        // USER1 claims a tradify nullifier
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // USER2 tries to use same tradify nullifier with different lender
        bytes32[] memory nullifiers2 = new bytes32[](1);
        nullifiers2[0] = DIFFERENT_LENDER;

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER2,
            nullifiers2,
            "tradify1"
        );

        vm.prank(USER2);
        vm.expectRevert("User tries to use not his tradify score.");
        creditContract.submitTEECreditScore(journalData2, "attestation2");
    }

    function testAddNullifiers_UsedOwnedAccount_Reverts() public {
        // USER1 uses an owned account
        bytes32[] memory nullifiers1 = new bytes32[](2);
        nullifiers1[0] = LENDER_NULLIFIER;
        nullifiers1[1] = OWNED_NULLIFIER_1;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // USER2 tries to use the same owned account
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = DIFFERENT_LENDER;
        nullifiers2[1] = OWNED_NULLIFIER_1; // Already used by USER1

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER2,
            nullifiers2,
            "tradify2"
        );

        vm.prank(USER2);
        vm.expectRevert(
            "User tries to use ethAccount for his maxcredit score calculation, that is already in use."
        );
        creditContract.submitTEECreditScore(journalData2, "attestation2");
    }

    function testAddNullifiers_UsedLenderAccount_Reverts() public {
        // USER1 uses a lender account
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // USER2 tries to use the same lender account as owned account
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = DIFFERENT_LENDER;
        nullifiers2[1] = LENDER_NULLIFIER; // Already used by USER1 as lender

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER2,
            nullifiers2,
            "tradify2"
        );

        vm.prank(USER2);
        vm.expectRevert(
            "User tries to use ethAccount for his maxcredit score calculation, that is already in use."
        );
        creditContract.submitTEECreditScore(journalData2, "attestation2");
    }

    // ============= DELETE NULLIFIERS TESTS =============

    function testDeleteNullifiers_UpdateFreesOldNullifiers() public {
        // Initial submission with multiple nullifiers
        bytes32[] memory nullifiers1 = new bytes32[](3);
        nullifiers1[0] = LENDER_NULLIFIER;
        nullifiers1[1] = OWNED_NULLIFIER_1;
        nullifiers1[2] = OWNED_NULLIFIER_2;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // Verify all are used initially
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender nullifier should be used initially"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "Owned nullifier 1 should be used initially"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_2),
            "Owned nullifier 2 should be used initially"
        );

        // Update with fewer nullifiers - should free the unused ones
        bytes32[] memory nullifiers2 = new bytes32[](1);
        nullifiers2[0] = LENDER_NULLIFIER; // Keep only lender

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER1,
            nullifiers2,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData2, "attestation2");

        // Old owned nullifiers should be freed
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender nullifier should still be used"
        );
        assertFalse(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "Owned nullifier 1 should be freed"
        );
        assertFalse(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_2),
            "Owned nullifier 2 should be freed"
        );
    }

    function testDeleteNullifiers_CompletelyNewSet() public {
        // Initial submission
        bytes32[] memory nullifiers1 = new bytes32[](2);
        nullifiers1[0] = LENDER_NULLIFIER;
        nullifiers1[1] = OWNED_NULLIFIER_1;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // Update with completely different nullifiers
        bytes32[] memory nullifiers2 = new bytes32[](1);
        nullifiers2[0] = keccak256("new_lender");

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER1,
            nullifiers2,
            "tradify2"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData2, "attestation2");

        // All old nullifiers should be freed
        assertFalse(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Old lender nullifier should be freed"
        );
        assertFalse(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "Old owned nullifier should be freed"
        );

        // New nullifier should be used
        assertTrue(
            creditContract.usedAccountsNullifiers(keccak256("new_lender")),
            "New lender nullifier should be used"
        );
    }

    function testDeleteNullifiers_NoExistingScore() public {
        // Try to submit for user with no existing score (should fail in deleteOldNullifiers)
        bytes32[] memory nullifiers = new bytes32[](1);
        nullifiers[0] = LENDER_NULLIFIER;

        CreditScore.JournalData memory journalData = createJournalData(
            USER1,
            nullifiers,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData, "attestation");
    }

    // ============= NULLIFIER REUSE TESTS =============

    function testNullifierReuse_AfterFreeing() public {
        // USER1 uses some nullifiers
        bytes32[] memory nullifiers1 = new bytes32[](2);
        nullifiers1[0] = LENDER_NULLIFIER;
        nullifiers1[1] = OWNED_NULLIFIER_1;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // USER1 updates and frees OWNED_NULLIFIER_1
        bytes32[] memory nullifiers2 = new bytes32[](1);
        nullifiers2[0] = LENDER_NULLIFIER; // Drop OWNED_NULLIFIER_1

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER1,
            nullifiers2,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData2, "attestation2");

        // Verify OWNED_NULLIFIER_1 is freed
        assertFalse(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "Owned nullifier should be freed after update"
        );

        // Now USER2 should be able to use the freed nullifier
        bytes32[] memory nullifiers3 = new bytes32[](1);
        nullifiers3[0] = OWNED_NULLIFIER_1; // Should now be available

        CreditScore.JournalData memory journalData3 = createJournalData(
            USER2,
            nullifiers3,
            "tradify2"
        );

        vm.prank(USER2);
        creditContract.submitTEECreditScore(journalData3, "attestation3");

        // Verify nullifier is now used by USER2
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "Freed nullifier should now be used by USER2"
        );
    }

    function testSameTradifyNullifier_SameUser() public {
        // Initial submission
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // Same user reuses same tradify nullifier (should work)
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = LENDER_NULLIFIER;
        nullifiers2[1] = OWNED_NULLIFIER_1;

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER1,
            nullifiers2,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData2, "attestation2");

        // Should succeed
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender should still be used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "New owned account should be used"
        );
        assertEq(
            creditContract.tradifyNullifiers("tradify1"),
            LENDER_NULLIFIER,
            "Tradify mapping should remain unchanged"
        );
    }

    // ============= EDGE CASES =============

    function testAddNullifiers_EmptyArray_Reverts() public {
        bytes32[] memory emptyNullifiers = new bytes32[](0);

        CreditScore.JournalData memory journalData = createJournalData(
            USER1,
            emptyNullifiers,
            "tradify1"
        );

        vm.prank(USER1);
        // Should revert due to array bounds access
        vm.expectRevert();
        creditContract.submitTEECreditScore(journalData, "attestation");
    }

    function testComplex_MultiUserNullifierManagement() public {
        // USER1 uses multiple nullifiers
        bytes32[] memory nullifiers1 = new bytes32[](3);
        nullifiers1[0] = LENDER_NULLIFIER;
        nullifiers1[1] = OWNED_NULLIFIER_1;
        nullifiers1[2] = OWNED_NULLIFIER_2;

        CreditScore.JournalData memory journalData1 = createJournalData(
            USER1,
            nullifiers1,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData1, "attestation1");

        // USER1 updates, freeing OWNED_NULLIFIER_2
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = LENDER_NULLIFIER;
        nullifiers2[1] = OWNED_NULLIFIER_1; // Keep this, free OWNED_NULLIFIER_2

        CreditScore.JournalData memory journalData2 = createJournalData(
            USER1,
            nullifiers2,
            "tradify1"
        );

        vm.prank(USER1);
        creditContract.submitTEECreditScore(journalData2, "attestation2");

        // USER2 uses the freed nullifier
        bytes32[] memory nullifiers3 = new bytes32[](1);
        nullifiers3[0] = OWNED_NULLIFIER_2; // This should be free now

        CreditScore.JournalData memory journalData3 = createJournalData(
            USER2,
            nullifiers3,
            "tradify2"
        );

        vm.prank(USER2);
        creditContract.submitTEECreditScore(journalData3, "attestation3");

        // Final state verification
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "USER1 lender should be used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "USER1 owned account should be used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_2),
            "USER2 should now use the freed nullifier"
        );

        // Verify tradify mappings
        assertEq(
            creditContract.tradifyNullifiers("tradify1"),
            LENDER_NULLIFIER,
            "USER1 tradify mapping"
        );
        assertEq(
            creditContract.tradifyNullifiers("tradify2"),
            OWNED_NULLIFIER_2,
            "USER2 tradify mapping"
        );
    }
}
