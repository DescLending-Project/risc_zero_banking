// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./risc0/ImageID.sol";
import "forge-std/console2.sol";

contract CreditScore {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public constant imageId = ImageID.GUEST_ID;
    
    /// @notice Authorized servers that can provide TradFi data
    mapping(string => bool) public authorizedServers;
    
    /// @notice Stored credit scores by user address
    mapping(address => CreditScoreData) public creditScores;
    
    /// @notice Credit score data structure
    struct CreditScoreData {
        uint64 score;           // Hybrid credit score (0-850)
        string serverName;      // TradFi data source server
        uint256 timestamp;      // When the score was submitted
        bool isValid;           // Whether the score is currently valid
    }
    
    /// @notice Events
    event CreditScoreSubmitted(
        address indexed user,
        uint64 score,
        string serverName,
        uint256 timestamp
    );
    
    event ServerAuthorized(string serverName, bool authorized);
    
    /// @notice Debug event showing key verification values
    event VerificationDebug(bytes32 imageId, bytes32 journalHash, uint256 sealLength);
    
    constructor(IRiscZeroVerifier _verifier) {
        verifier = _verifier;
        
        // Pre-authorize some common servers
        authorizedServers["httpbin.org"] = true;
        authorizedServers["schufa.com"] = true;
        authorizedServers["schufa.de"] = true;
    }
    
    /// @notice Submit a verified credit score
    /// @param score The calculated hybrid credit score (0-850)
    /// @param serverName The TradFi server that provided the data
    /// @param seal The RISC Zero proof seal
    function submitCreditScore(
        uint64 score,
        string calldata serverName,
        bytes calldata seal
    ) external {
        // Verify the server is authorized
        require(authorizedServers[serverName], "Server not authorized");
        
        // Compute journal hash from the score and serverName (matching guest output format)
        bytes32 journalHash = sha256(abi.encodePacked(score, serverName));
        
        console2.log("ImageID:", uint256(imageId));
        console2.log("Journal Hash:", uint256(journalHash));
        console2.log("Score:", score);
        console2.log("Server:", serverName);
        console2.log("Seal length:", seal.length);
        
        emit VerificationDebug(imageId, journalHash, seal.length);
        
        // Verify the RISC Zero proof
        verifier.verify(seal, imageId, journalHash);
        
        // Store the verified credit score
        creditScores[msg.sender] = CreditScoreData({
            score: score,
            serverName: serverName,
            timestamp: block.timestamp,
            isValid: true
        });
        
        emit CreditScoreSubmitted(msg.sender, score, serverName, block.timestamp);
    }
    
    /// @notice Get a user's credit score
    /// @param user The user's address
    /// @return The user's credit score data
    function getCreditScore(address user) external view returns (CreditScoreData memory) {
        return creditScores[user];
    }
    
    /// @notice Check if a user has a valid credit score above a threshold
    /// @param user The user's address
    /// @param minScore The minimum required score
    /// @return Whether the user meets the criteria
    function hasValidScore(address user, uint64 minScore) external view returns (bool) {
        CreditScoreData memory data = creditScores[user];
        return data.isValid && data.score >= minScore;
    }
    
    /// @notice Authorize or deauthorize a TradFi server
    /// @param serverName The server name to authorize/deauthorize
    /// @param authorized Whether the server should be authorized
    function setServerAuthorization(string calldata serverName, bool authorized) external {
        // In production, add proper access control
        authorizedServers[serverName] = authorized;
        emit ServerAuthorized(serverName, authorized);
    }
    
    /// @notice Invalidate a user's credit score (e.g., if it's too old)
    /// @param user The user's address
    function invalidateScore(address user) external {
        // In production, add proper access control
        creditScores[user].isValid = false;
    }
}