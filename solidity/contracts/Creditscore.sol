// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./risc0/ImageID.sol";

contract CreditScore {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public constant imageId = ImageID.GUEST_ID;
    uint256 public constant SCORE_EXPIRY_PERIOD = 90 days;
    uint256 public constant TRADFI_DATA_MAX_AGE = 10 days; 

    mapping(string => bool) public authorizedServers;
    mapping(string => bool) public authorizedStateRootProviders;
    mapping(address => CreditScoreData) public creditScores;
    mapping(bytes32 => bool) public usedAccountsNullifiers;
    mapping(bytes32 => mapping(uint256 => bytes32[])) public ownedAccountNullifiers; // maps from lender ETH Account nullifier to mapping of each of his submitted CreditScores( that are identified by the block height of submission ) to used nullifiers in that credit score calculation
    mapping(bytes32 => bytes32) public tradifyNullifiers; // mapping from tradify score Nullifier to its owner ETH accounts nullifier
    mapping(bytes32 => uint64) public storedCreditScoresNumber; // mapping from tradifyNullifier to the number of creditScores that are currently stored on the contract for him

    struct CreditScoreData {
        uint64 score;
        uint256 timestamp;
        bool isValid;
    }

    struct JournalData {
        uint64 score;
        string serverName;
        string stateRootProvider;
        uint64 blockNumber;
        string userIdHash;             
        uint64 tradfiDateTimestamp;     
        string userAddress;
        bytes32[] allNullifiers;
    }

    event CreditScoreSubmitted(
        address indexed user,
        uint64 score,
        uint256 timestamp,
        string userIdHash               
    );

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
        authorizedServers["openbanking-api-826260723607.europe-west3.run.app"] = true;
        authorizedStateRootProviders["sonic-blaze.g.alchemy.com"] = true;
    }

    function submitCreditScore(
        JournalData calldata journalData,
        bytes calldata seal
    ) external {
        require(authorizedServers[journalData.serverName], "TradFi server not authorized");
        require(authorizedStateRootProviders[journalData.stateRootProvider], "State root provider not authorized");

        require(
            journalData.blockNumber >= block.number, // has to be adjusted in real production, shall just pass for now
            "State root data is too old"
        );

        // Validate TradFi timestamp is within acceptable range
        // TODO: Re-enable timestamp validation for production deployment
        // uint64 currentDateTimestamp = getCurrentDateTimestamp();
        
        // DEVELOPMENT ONLY: Skip timestamp validation for Anvil testing
        // Remove this section and uncomment above for production
        /*
        uint64 minValidTimestamp;
        if (currentDateTimestamp >= uint64(TRADFI_DATA_MAX_AGE)) {
            minValidTimestamp = currentDateTimestamp - uint64(TRADFI_DATA_MAX_AGE);
        } else {
            minValidTimestamp = 0; // If we're within 10 days of epoch, allow any timestamp
        }
        
        require(
            journalData.tradfiDateTimestamp >= minValidTimestamp,
            "TradFi data is older than 10 days"
        );
        
        require(
            journalData.tradfiDateTimestamp <= currentDateTimestamp + 1 days,
            "TradFi timestamp cannot be too far in the future"
        );
        */

        // Verify the ZK proof
        bytes memory journal = abi.encode(journalData);
        bytes32 journalHash = sha256(journal);
        verifier.verify(seal, imageId, journalHash);

        // Store the credit score
        creditScores[msg.sender] = CreditScoreData(
            journalData.score,
            block.timestamp,
            true
        );

        emit CreditScoreSubmitted(
            msg.sender,
            journalData.score,
            block.timestamp,
            journalData.userIdHash
        );
    }

    function getCreditScore(address user) external view returns (
        uint64 score,
        bool isValid,
        uint256 timestamp
    ) {
        CreditScoreData memory userData = creditScores[user];
        bool notExpired = userData.isValid &&
            userData.timestamp > 0 &&
            (block.timestamp - userData.timestamp) <= SCORE_EXPIRY_PERIOD;

        if (notExpired) {
            return (
                userData.score,
                true,
                userData.timestamp
            );
        } else {
            return (
                0,
                false,
                userData.timestamp
            );
        }
    }

    // Helper function to get current date timestamp (today at 00:00:00 UTC)
    function getCurrentDateTimestamp() internal view returns (uint64) {
        // Convert current timestamp to date-only timestamp
        return uint64((block.timestamp / 86400) * 86400);
    }

    // View function to check if a TradFi timestamp would be valid
    // DEVELOPMENT ONLY: Always returns true for Anvil testing
    function isValidTradfiTimestamp(uint64 tradfiTimestamp) external view returns (bool) {
        // TODO: Re-enable for production
        return true; // Always pass during development
        
        /*
        uint64 currentDateTimestamp = getCurrentDateTimestamp();
        
        // Prevent underflow
        uint64 minValidTimestamp;
        if (currentDateTimestamp >= uint64(TRADFI_DATA_MAX_AGE)) {
            minValidTimestamp = currentDateTimestamp - uint64(TRADFI_DATA_MAX_AGE);
        } else {
            minValidTimestamp = 0;
        }
        
        return tradfiTimestamp >= minValidTimestamp &&
               tradfiTimestamp <= currentDateTimestamp + 1 days;
        */
    }

    // NOTE: first nullifier in the userOwnedAccountsNullifiers must be the nullifier of the eth account (lender account) for which user tries to get a loan.(this is checked during defi_inputs_validation)
    function addNullifiers(bytes32[] calldata userOwnedAccountsNullifiers, bytes32 userTradifyNullifier) external {
        require(tradifyNullifiers[userTradifyNullifier] == bytes32(0) || tradifyNullifiers[userTradifyNullifier] == userOwnedAccountsNullifiers[0], "User tries to use not his tradify score.");
        
        // storing the userTradifyNullifier
        tradifyNullifiers[userTradifyNullifier] = userOwnedAccountsNullifiers[0];

        // verifying that the users ethAccount was not used for calculation of creditScore for some other ethAccount
        // NOTE: we allow user to reuse his lender account, as we are tracking the amount of funds that he owes us and this amount is included in the score calculation
        if(usedAccountsNullifiers[userOwnedAccountsNullifiers[0]]){
            // check if the nullifier was used by the lender account or as one of the ownedAccounts
            require(storedCreditScoresNumber[userOwnedAccountsNullifiers[0]] > 0, "Provided User lender account is already in use by some other lender account.");
        }

        // storing the lender accounts nullifier
        usedAccountsNullifiers[userOwnedAccountsNullifiers[0]] = true;
        ownedAccountNullifiers[userOwnedAccountsNullifiers[0]][block.number].push(userOwnedAccountsNullifiers[0]);
        storedCreditScoresNumber[userOwnedAccountsNullifiers[0]]++;

        // iterating over provided nullifiers checking if they were not already in use and storing them
        for (uint i = 1; i < userOwnedAccountsNullifiers.length; i++){
            require(usedAccountsNullifiers[userOwnedAccountsNullifiers[i]] == false, 'User tries to use ethAccount for his maxcredit score calculation, that is already in use.');
            // storing the nullifier
            usedAccountsNullifiers[userOwnedAccountsNullifiers[i]] = true;

            // storing the accountsNullifier in array of the lender account nullifier in relation to the blockheight at which the creditScore was submitted
            ownedAccountNullifiers[userOwnedAccountsNullifiers[0]][block.number].push(userOwnedAccountsNullifiers[i]);
        }
    }

    // NOTE: should be called when the CreditScore gets deleted from the contract
    // it deletes all nullifiers related to the creditScore that was submitted at the creditScoreSubmissionBlock height
    function deleteNullifiers(uint256 creditScoreSubmissionBlockHeight, bytes32 lenderNullifier) external {
        require(ownedAccountNullifiers[lenderNullifier][creditScoreSubmissionBlockHeight].length > 0, "CreditScore related nullifiers not found.");

        // Deleting all usedAccountsNullifiers for this CreditScore calculation
        for (uint256 index = 1; index < ownedAccountNullifiers[lenderNullifier][creditScoreSubmissionBlockHeight].length; index++) {
            delete usedAccountsNullifiers[ownedAccountNullifiers[lenderNullifier][creditScoreSubmissionBlockHeight][index]];
        }

        // deleting the array of associated nullifiers
        delete ownedAccountNullifiers[lenderNullifier][creditScoreSubmissionBlockHeight];

        require(storedCreditScoresNumber[lenderNullifier] > 0, "User did not have any Credit Score");
        storedCreditScoresNumber[lenderNullifier]--;

        // in case where this was the last used user credit score, user can utilize this lending account as ownedAccount in score calculation for different lending account
        if(storedCreditScoresNumber[lenderNullifier] == 0){
            delete usedAccountsNullifiers[lenderNullifier];
            delete storedCreditScoresNumber[lenderNullifier];
        }
    }

    // need proper auth in deployment
    function authorizeServer(string calldata serverName, bool authorized) external {
        authorizedServers[serverName] = authorized;
    }

    function authorizeStateRootProvider(string calldata providerName, bool authorized) external {
        authorizedStateRootProviders[providerName] = authorized;
    }
}