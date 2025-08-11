// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./risc0/ImageID.sol";

contract CreditScore {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public constant imageId = ImageID.GUEST_ID;
    uint256 public constant TRADIFY_DATA_MAX_AGE = 1 days;
    uint256 public constant BLCOKCHAIN_DATA_MAX_AGE = 7200; // this is about one day

    mapping(string => bool) public authorizedServers;
    mapping(string => bool) public authorizedStateRootProviders;
    mapping(address => CreditScoreData) public creditScores;
    mapping(bytes32 => bool) public usedAccountsNullifiers;
    mapping(bytes32 => bytes32) public tradfiNullifiers; // mapping from tradify score Nullifier to its owner ETH accounts nullifier

    struct CreditScoreData {
        uint64 score;
        uint256 timestamp;
        bool isUnused;
        bytes32[] nullifiers;
    }

    struct JournalData {
        uint64 score;
        string serverName;
        string stateRootProvider;
        uint64 blockNumber;
        bytes32 tradfiNullifier; 
        uint64 tradfiDateTimestamp;
        address userAddress; 
        bytes32[] allNullifiers;
    }

    event CreditScoreSubmitted(
        address indexed user,
        uint64 score,
        uint256 timestamp,
        bytes32 tradfiNullifier
    );

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
        authorizedServers[
            "openbanking-api-826260723607.europe-west3.run.app"
        ] = true;
        authorizedStateRootProviders["supertrusworthynodeprovider.jermatek.com"] = true;
    }

    // Validates the content of credit Score journalData
    // 1. Checks if if the tradify score and stateRoot came from authorizedSources
    // 2. Checks the fresshnes of the calculated data
    // 3. Check and store  the creditScore related account nullifiers
    function validateCreditScoreData(
        JournalData calldata journalData
    ) internal {
        require(
            authorizedServers[journalData.serverName],
            "TradFi server not authorized"
        );
        require(
            authorizedStateRootProviders[journalData.stateRootProvider],
            "State root provider not authorized"
        );
/*
        require(
            block.timestamp - journalData.tradfiDateTimestamp <=
                TRADIFY_DATA_MAX_AGE,
            " Tradify data is to old"
        );
*//*
        require(
            block.number - journalData.blockNumber <= BLCOKCHAIN_DATA_MAX_AGE,
            "Blockchain data is to old"
        );
*/
        deleteOldNullifiers(journalData.userAddress);
        addNewNullifiers(
            journalData.allNullifiers,
            journalData.tradfiNullifier,
            journalData.userAddress
        );
    }

    function submitTEECreditScore(
        JournalData calldata journalData,
        bytes calldata attestation
    ) external {
        // TODO: add the attestation verificcation call
        require(attestation.length > 0, "Attestation needs to be provided");

        validateCreditScoreData(journalData);
        // Store the credit score
        creditScores[msg.sender] = CreditScoreData(
            journalData.score,
            block.timestamp,
            false,
            journalData.allNullifiers
        );

        emit CreditScoreSubmitted(
            msg.sender,
            journalData.score,
            block.timestamp,
            journalData.tradfiNullifier
        );
    }

   
    function submitR0CreditScore(
        JournalData calldata journalData,
        bytes calldata seal
    ) external {
        // Verify the ZK proof
        bytes memory journal = abi.encode(journalData);
        bytes32 journalHash = sha256(journal);
        verifier.verify(seal, imageId, journalHash);

        validateCreditScoreData(journalData);

        // Store the credit score
        creditScores[msg.sender] = CreditScoreData(
            journalData.score,
            block.timestamp,
            false,
            journalData.allNullifiers
        );

        emit CreditScoreSubmitted(
            msg.sender,
            journalData.score,
            block.timestamp,
            journalData.tradfiNullifier
        );
    }

    function getCreditScore(
        address user
    ) external view returns (uint64 score, bool isUnused, uint256 timestamp) {
        return (
            creditScores[user].score,
            creditScores[user].isUnused,
            creditScores[user].timestamp
        );
    }

    function markCreditScoreAsUsed(address user) external {
        creditScores[user].isUnused = false;
    }

    function addNewNullifiers(
        bytes32[] calldata userOwnedAccountsNullifiers,
        bytes32 usertradfiNullifier,
        address lenderAddress
    ) internal {
        require(
            tradfiNullifiers[usertradfiNullifier] == bytes32(0) ||
                tradfiNullifiers[usertradfiNullifier] ==
                userOwnedAccountsNullifiers[0],
            "User tries to use not his tradify score."
        );

        // storing the usertradfiNullifier in relation to his lending acount nullifier
        tradfiNullifiers[usertradfiNullifier] = userOwnedAccountsNullifiers[
            0
        ];

        // verifying that the users ethAccount was not used for calculation of creditScore for some other ethAccount
        if (usedAccountsNullifiers[userOwnedAccountsNullifiers[0]]) {
            // check if the nullifier was used by the lender account or as one of the ownedAccounts by sombody else
            require(
                creditScores[lenderAddress].nullifiers[0] ==
                    userOwnedAccountsNullifiers[0],
                "Provided User lender account is already in use by some other lender account."
            );
        }

        // storing the lender accounts nullifier
        usedAccountsNullifiers[userOwnedAccountsNullifiers[0]] = true;

        // iterating over provided nullifiers checking if they were not already in use and storing them
        for (uint i = 1; i < userOwnedAccountsNullifiers.length; i++) {
            require(
                usedAccountsNullifiers[userOwnedAccountsNullifiers[i]] == false,
                "User tries to use ethAccount for his maxcredit score calculation, that is already in use."
            );
            // storing the nullifier
            usedAccountsNullifiers[userOwnedAccountsNullifiers[i]] = true;
        }
    }

    // NOTE: should be called when the CreditScore gets deleted from the contract
    // it deletes all nullifiers related to the creditScore that was submitted at the creditScoreSubmissionBlock height
    function deleteOldNullifiers(address lenderAddress) internal {
        if (creditScores[lenderAddress].nullifiers.length <= 0) {
            return;
        }

        // Deleting all usedAccountsNullifiers for this CreditScore calculation
        for (
            uint256 index = 0;
            index < creditScores[lenderAddress].nullifiers.length;
            index++
        ) {
            delete usedAccountsNullifiers[
                creditScores[lenderAddress].nullifiers[index]
            ];
        }
    }

    // need proper auth in deployment
    function authorizeServer(
        string calldata serverName,
        bool authorized
    ) external {
        authorizedServers[serverName] = authorized;
    }

    function authorizeStateRootProvider(
        string calldata providerName,
        bool authorized
    ) external {
        authorizedStateRootProviders[providerName] = authorized;
    }
}