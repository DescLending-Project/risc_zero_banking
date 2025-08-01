// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./risc0/ImageID.sol";

contract CreditScore {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public constant imageId = ImageID.GUEST_ID;
    
    uint256 public constant SCORE_EXPIRY_PERIOD = 90 days;
    
    mapping(string => bool) public authorizedServers;
    mapping(string => bool) public authorizedStateRootProviders;
    mapping(address => CreditScoreData) public creditScores;

    struct CreditScoreData {
        uint64 score;
        string serverName;
        string stateRootProvider;
        uint256 timestamp;
        bool isValid;
    }

    event CreditScoreSubmitted(
        address indexed user,
        uint64 score,
        string serverName,
        string stateRootProvider,
        uint256 timestamp
    );
    event ServerAuthorized(string serverName, bool authorized);
    event StateRootProviderAuthorized(string providerName, bool authorized);

    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;

        authorizedServers["httpbin.org"] = true;
        authorizedServers["openbanking-api-826260723607.europe-west3.run.ap"] = true; // last "p" is missing as domain is to long even with 128 byte journal
        authorizedServers["schufa.de"] = true;
        
        authorizedStateRootProviders["sonic-blaze.g.alchemy.com"] = true;
        authorizedStateRootProviders["infura.com"] = true;
    }

    function submitCreditScore(
        uint64 score,
        string calldata serverName,
        string calldata stateRootProvider,
        bytes calldata seal,
        bytes calldata journalData
    ) external {
        require(authorizedServers[serverName], "TradFi server not authorized");
        require(authorizedStateRootProviders[stateRootProvider], "State root provider not authorized");

        bytes32 journalHash = sha256(journalData);
        verifier.verify(seal, imageId, journalHash);

        creditScores[msg.sender] = CreditScoreData({
            score: score,
            serverName: serverName,
            stateRootProvider: stateRootProvider,
            timestamp: block.timestamp,
            isValid: true
        });

        emit CreditScoreSubmitted(msg.sender, score, serverName, stateRootProvider, block.timestamp);
    }

    function getCreditScore(address user) external view returns (
        uint64 score,
        bool isValid,
        uint256 timestamp
    ) {
        CreditScoreData memory userData = creditScores[user];
        
        // Check if score exists and is not expired
        bool notExpired = userData.isValid && 
                         userData.timestamp > 0 && 
                         (block.timestamp - userData.timestamp) <= SCORE_EXPIRY_PERIOD;
        
        if (notExpired) {
            return (userData.score, true, userData.timestamp);
        } else {
            return (0, false, userData.timestamp);
        }
    }

    function authorizeServer(string calldata serverName, bool authorized) external {
        authorizedServers[serverName] = authorized;
        emit ServerAuthorized(serverName, authorized);
    }

    function authorizeStateRootProvider(string calldata providerName, bool authorized) external {
        authorizedStateRootProviders[providerName] = authorized;
        emit StateRootProviderAuthorized(providerName, authorized);
    }

    function testVerify(
        bytes calldata seal,
        bytes calldata journalData
    ) external view returns (bool) {
        bytes32 journalHash = sha256(journalData);
        verifier.verify(seal, imageId, journalHash);
        return true;
    }

    function isServerAuthorized(string calldata serverName) external view returns (bool) {
        return authorizedServers[serverName];
    }

    function isStateRootProviderAuthorized(string calldata providerName) external view returns (bool) {
        return authorizedStateRootProviders[providerName];
    }
}