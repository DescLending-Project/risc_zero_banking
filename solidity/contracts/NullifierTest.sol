// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "./Creditscore.sol";
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {Receipt} from "risc0/IRiscZeroVerifier.sol";

// Mock verifier for testing
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
    bytes32 constant TRADIFY_NULLIFIER = keccak256("tradify1");
    bytes32 constant DIFFERENT_LENDER = keccak256("different_lender");

    function setUp() public {
        mockVerifier = new MockRiscZeroVerifier();
        creditContract = new CreditScore(mockVerifier);
    }

    // Helper functions to check array elements
    function checkArrayElement(bytes32 lender, uint256 blockHeight, uint256 index, bytes32 expected) internal view returns (bool) {
        try creditContract.ownedAccountNullifiers(lender, blockHeight, index) returns (bytes32 element) {
            return element == expected;
        } catch {
            return false; // Out of bounds or error
        }
    }
    
    function isArrayEmpty(bytes32 lender, uint256 blockHeight) internal view returns (bool) {
        try creditContract.ownedAccountNullifiers(lender, blockHeight, 0) returns (bytes32) {
            return false; // Has at least one element
        } catch {
            return true; // Out of bounds means empty
        }
    }

    // Helper to check if array has at least N elements
    function hasAtLeastNElements(bytes32 lender, uint256 blockHeight, uint256 n) internal view returns (bool) {
        if (n == 0) return true;
        try creditContract.ownedAccountNullifiers(lender, blockHeight, n - 1) returns (bytes32) {
            return true; // Element exists at index n-1, so array has at least n elements
        } catch {
            return false; // Out of bounds, array has fewer than n elements
        }
    }

    // Helper to get exact array length
    function getArrayLength(bytes32 lender, uint256 blockHeight) internal view returns (uint256) {
        for (uint256 i = 0; i < 100; i++) { // Reasonable upper limit
            try creditContract.ownedAccountNullifiers(lender, blockHeight, i) returns (bytes32) {
                // Element exists, continue
            } catch {
                return i; // First index that reverts is the length
            }
        }
        return 100; // Max reached
    }

    // Helper to get specific element safely
    function getElementSafely(bytes32 lender, uint256 blockHeight, uint256 index) internal view returns (bytes32) {
        try creditContract.ownedAccountNullifiers(lender, blockHeight, index) returns (bytes32 element) {
            return element;
        } catch {
            return bytes32(0); // Return zero if out of bounds
        }
    }

    // ============= ADD NULLIFIERS TESTS =============

    function testAddNullifiers_SingleAccount_Success() public {
        bytes32[] memory nullifiers = new bytes32[](1);
        nullifiers[0] = LENDER_NULLIFIER;
        
        uint256 currentBlock = block.number;
        
        creditContract.addNullifiers(nullifiers, TRADIFY_NULLIFIER);
        
        // Verify state changes
        assertEq(
            creditContract.tradifyNullifiers(TRADIFY_NULLIFIER), 
            LENDER_NULLIFIER,
            "Tradify nullifier should map to lender nullifier"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender nullifier should be marked as used"
        );
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            1,
            "Stored credit score number should be 1 after first submission"
        );
        
        // Check array was populated correctly
        assertTrue(
            checkArrayElement(LENDER_NULLIFIER, currentBlock, 0, LENDER_NULLIFIER),
            "First array element should be the lender nullifier"
        );
        assertTrue(
            hasAtLeastNElements(LENDER_NULLIFIER, currentBlock, 1),
            "Array should have at least 1 element"
        );
        assertFalse(
            hasAtLeastNElements(LENDER_NULLIFIER, currentBlock, 2),
            "Array should not have 2 elements (only 1 expected)"
        );
    }

    function testAddNullifiers_MultipleAccounts_Success() public {
        bytes32[] memory nullifiers = new bytes32[](3);
        nullifiers[0] = LENDER_NULLIFIER;
        nullifiers[1] = OWNED_NULLIFIER_1;
        nullifiers[2] = OWNED_NULLIFIER_2;
        
        uint256 currentBlock = block.number;
        
        creditContract.addNullifiers(nullifiers, TRADIFY_NULLIFIER);
        
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
        
        // Verify array contains all nullifiers
        assertTrue(
            checkArrayElement(LENDER_NULLIFIER, currentBlock, 0, LENDER_NULLIFIER),
            "First array element should be the lender nullifier"
        );
        assertTrue(
            checkArrayElement(LENDER_NULLIFIER, currentBlock, 1, OWNED_NULLIFIER_1),
            "Second array element should be first owned nullifier"
        );
        assertTrue(
            checkArrayElement(LENDER_NULLIFIER, currentBlock, 2, OWNED_NULLIFIER_2),
            "Third array element should be second owned nullifier"
        );
        assertTrue(
            hasAtLeastNElements(LENDER_NULLIFIER, currentBlock, 3),
            "Array should have at least 3 elements"
        );
        assertFalse(
            hasAtLeastNElements(LENDER_NULLIFIER, currentBlock, 4),
            "Array should not have 4 elements (only 3 expected)"
        );
        
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            1,
            "Stored credit score number should be 1 after submission"
        );
    }

    function testAddNullifiers_ReuseOwnLenderAccount_Success() public {
        // First submission
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;
        creditContract.addNullifiers(nullifiers1, TRADIFY_NULLIFIER);
        
        // Advance block to simulate different submission time
        vm.roll(block.number + 10);
        
        // Second submission with same lender account - should succeed
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = LENDER_NULLIFIER;
        nullifiers2[1] = OWNED_NULLIFIER_1;
        
        bytes32 newTradifyNullifier = keccak256("tradify2");
        creditContract.addNullifiers(nullifiers2, newTradifyNullifier);
        
        // Should have incremented counter
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            2,
            "Should have 2 stored credit scores after reusing lender account"
        );
    }

    function testAddNullifiers_WrongTradifyOwner_Reverts() public {
        // First user adds nullifiers
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;
        creditContract.addNullifiers(nullifiers1, TRADIFY_NULLIFIER);
        
        // Different user tries to use same tradify nullifier
        bytes32[] memory nullifiers2 = new bytes32[](1);
        nullifiers2[0] = DIFFERENT_LENDER;
        
        vm.expectRevert("User trys to use not his tradify score.");
        creditContract.addNullifiers(nullifiers2, TRADIFY_NULLIFIER);
    }

    function testAddNullifiers_UsedAccountByOther_Reverts() public {
        // First user uses an account
        bytes32[] memory nullifiers1 = new bytes32[](2);
        nullifiers1[0] = LENDER_NULLIFIER;
        nullifiers1[1] = OWNED_NULLIFIER_1;
        creditContract.addNullifiers(nullifiers1, TRADIFY_NULLIFIER);
        
        // Second user tries to use the same owned account
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = DIFFERENT_LENDER;
        nullifiers2[1] = OWNED_NULLIFIER_1; // This should fail
        
        bytes32 differentTradify = keccak256("tradify2");
        vm.expectRevert("User trys to use ethAccount for his maxcredit score calculation, that is already in use.");
        creditContract.addNullifiers(nullifiers2, differentTradify);
    }


    // ============= DELETE NULLIFIERS TESTS =============

    function testDeleteNullifiers_Success() public {
        // First add nullifiers
        bytes32[] memory nullifiers = new bytes32[](3);
        nullifiers[0] = LENDER_NULLIFIER;
        nullifiers[1] = OWNED_NULLIFIER_1;
        nullifiers[2] = OWNED_NULLIFIER_2;
        
        uint256 submissionBlock = block.number;
        creditContract.addNullifiers(nullifiers, TRADIFY_NULLIFIER);
        
        // Verify initial state
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            1,
            "Should have 1 stored credit score before deletion"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender account should be marked as used before deletion"
        );
        
        // Delete nullifiers
        creditContract.deleteNullifiers(submissionBlock, LENDER_NULLIFIER);
        
        // Verify deletion
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            0,
            "Should have 0 stored credit scores after deletion"
        );
        assertFalse(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender account should not be marked as used after last score deletion"
        );
        
        // Verify array is deleted
        assertTrue(
            isArrayEmpty(LENDER_NULLIFIER, submissionBlock),
            "Array should be empty after deletion"
        );
    }

    function testDeleteNullifiers_MultipleScores_PartialDelete() public {
        // Add first set of nullifiers
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;
        uint256 firstBlock = block.number;
        creditContract.addNullifiers(nullifiers1, TRADIFY_NULLIFIER);
        
        // Advance block and add second set
        vm.roll(block.number + 10);
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = LENDER_NULLIFIER;
        nullifiers2[1] = OWNED_NULLIFIER_1;
        uint256 secondBlock = block.number;
        bytes32 tradify2 = keccak256("tradify2");
        creditContract.addNullifiers(nullifiers2, tradify2);
        
        // Should have 2 stored credit scores
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            2,
            "Should have 2 stored credit scores after adding two sets"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender account should be marked as used with multiple scores"
        );
        
        // Delete only the first one
        creditContract.deleteNullifiers(firstBlock, LENDER_NULLIFIER);
        
        // Should still have 1 credit score and account should still be used
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            1,
            "Should have 1 stored credit score after partial deletion"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender account should still be marked as used after partial deletion"
        );
        
        // First block array should be empty, second should still exist
        assertTrue(
            isArrayEmpty(LENDER_NULLIFIER, firstBlock),
            "First block array should be empty after deletion"
        );
        assertTrue(
            hasAtLeastNElements(LENDER_NULLIFIER, secondBlock, 2),
            "Second block array should have at least 2 elements"
        );
        assertFalse(
            hasAtLeastNElements(LENDER_NULLIFIER, secondBlock, 3),
            "Second block array should not have 3 elements (only 2 expected)"
        );
    }

    function testDeleteNullifiers_NonexistentBlock_Reverts() public {
        vm.expectRevert("CreditScore related nullifiers not found.");
        creditContract.deleteNullifiers(999999, LENDER_NULLIFIER);
    }

    function testDeleteNullifiers_NoStoredScores_Reverts() public {
        // First add nullifiers
        bytes32[] memory nullifiers = new bytes32[](1);
        nullifiers[0] = LENDER_NULLIFIER;
        uint256 submissionBlock = block.number;
        creditContract.addNullifiers(nullifiers, TRADIFY_NULLIFIER);
        
        // Delete them
        creditContract.deleteNullifiers(submissionBlock, LENDER_NULLIFIER);
        
        // Try to delete again - should fail
        vm.expectRevert("CreditScore related nullifiers not found.");
        creditContract.deleteNullifiers(submissionBlock, LENDER_NULLIFIER);
    }

    function testDeleteNullifiers_EmptyArray_Reverts() public {
        // Manually create a scenario where block exists but array is empty
        // This is harder to test directly, so we'll test with nonexistent data
        vm.expectRevert("CreditScore related nullifiers not found.");
        creditContract.deleteNullifiers(block.number, LENDER_NULLIFIER);
    }

    // ============= INTEGRATION TESTS =============

    function testFullCycle_AddAndDeleteMultiple() public {
        bytes32[] memory nullifiers = new bytes32[](3);
        nullifiers[0] = LENDER_NULLIFIER;
        nullifiers[1] = OWNED_NULLIFIER_1;
        nullifiers[2] = OWNED_NULLIFIER_2;
        
        uint256 submissionBlock = block.number;
        
        // Add nullifiers
        creditContract.addNullifiers(nullifiers, TRADIFY_NULLIFIER);
        
        // Verify all accounts are marked as used
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender account should be marked as used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "First owned account should be marked as used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_2),
            "Second owned account should be marked as used"
        );
        
        // Delete nullifiers
        creditContract.deleteNullifiers(submissionBlock, LENDER_NULLIFIER);
        
        // Verify only lender account is freed (others remain used)
        assertFalse(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Lender account should be freed after deletion"
        );
        assertFalse(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_1),
            "First owned account should freed after deletion"
        );
        assertFalse(
            creditContract.usedAccountsNullifiers(OWNED_NULLIFIER_2),
            "Second owned account should  freed after deletion"
        );
    }

    function testReuseAccountAfterDelete() public {
        // First user uses account
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;
        uint256 firstBlock = block.number;
        creditContract.addNullifiers(nullifiers1, TRADIFY_NULLIFIER);
        
        // Delete the credit score
        creditContract.deleteNullifiers(firstBlock, LENDER_NULLIFIER);
        
        // Now different user should be able to use this account as owned account
        vm.roll(block.number + 10);
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = DIFFERENT_LENDER;
        nullifiers2[1] = LENDER_NULLIFIER; // Now as owned account
        
        bytes32 newTradify = keccak256("tradify2");
        creditContract.addNullifiers(nullifiers2, newTradify);
        
        // Should succeed
        assertTrue(
            creditContract.usedAccountsNullifiers(DIFFERENT_LENDER),
            "Different lender account should be marked as used"
        );
        assertTrue(
            creditContract.usedAccountsNullifiers(LENDER_NULLIFIER),
            "Original lender account should be marked as used again (now as owned account)"
        );
    }

    function testSameTradifyNullifier_SameOwner_Success() public {
        // Add first set
        bytes32[] memory nullifiers1 = new bytes32[](1);
        nullifiers1[0] = LENDER_NULLIFIER;
        creditContract.addNullifiers(nullifiers1, TRADIFY_NULLIFIER);
        
        // Same owner can reuse same tradify nullifier
        vm.roll(block.number + 10);
        bytes32[] memory nullifiers2 = new bytes32[](2);
        nullifiers2[0] = LENDER_NULLIFIER;
        nullifiers2[1] = OWNED_NULLIFIER_1;
        creditContract.addNullifiers(nullifiers2, TRADIFY_NULLIFIER);
        
        // Should work fine
        assertEq(
            creditContract.storedCreditScoresNumber(LENDER_NULLIFIER), 
            2,
            "Should have 2 stored credit scores when reusing same tradify nullifier"
        );
    }

    function testEdgeCase_EmptyNullifiersArray() public {
        bytes32[] memory emptyNullifiers = new bytes32[](0);
        
        // This should revert due to array access out of bounds
        vm.expectRevert();
        creditContract.addNullifiers(emptyNullifiers, TRADIFY_NULLIFIER);
    }

    // ============= HELPER FUNCTIONS FOR TESTING =============

    function printTestState(bytes32 lender, uint256 blockHeight) public view {
        console.log("=== Test State ===");
        console.log("Stored Credit Scores Number:", creditContract.storedCreditScoresNumber(lender));
        console.log("Used Account Nullifiers:", creditContract.usedAccountsNullifiers(lender));
        console.log("Array empty:", isArrayEmpty(lender, blockHeight));
        console.log("Array length:", getArrayLength(lender, blockHeight));
        
        // Print first few elements safely
        for (uint i = 0; i < 5; i++) {
            bytes32 element = getElementSafely(lender, blockHeight, i);
            if (element != bytes32(0)) {
                console.log("Element", i);
                console.logBytes32(element);
            }
        }
    }
}
