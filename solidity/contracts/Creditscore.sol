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
    mapping(bytes32 => bool) public usedAccountsNullifiers; // 
    mapping(bytes32 => mapping(uint256 => bytes32[])) public ownedAccountNullifiers; // maps from lender  ETH Account nullifier to mapping of  each of his submited CreditSocres( that are identified by the block height of submision ) to used nullifiers in that credit score calcultaion
    mapping(bytes32 => bytes32) public tradifyNullifiers; // mapping from tradify score  Nullifier to its owner ETH accounts nullifier
    mapping(bytes32 => uint64) public storedCreditScoresNumber; // mapping from tradifyNullifier to the number  of creditScores that are curently stored on the contract for him


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

    // NOTE: first nullifier in the userOwnedAccountsNullifiers must be the nullifier of the eth account (lender account) for witch user trys to get a loan.(this is checked during defi_inputs_validation)
    function addNullifiers(bytes32[] calldata userOwnedAccountsNullifiers , bytes32 userTradifyNullifier) external{

      require(tradifyNullifiers[userTradifyNullifier] == bytes32(0) || tradifyNullifiers[userTradifyNullifier] == userOwnedAccountsNullifiers[0], "User trys to use not his tradify score.");
      // storing the userTradifyNullifer
      tradifyNullifiers[userTradifyNullifier] = userOwnedAccountsNullifiers[0];




      // verifing that the users ethAccount was not used for calcultaion of creditScore for some other ethAccount
      // NOTE: we allow user to reuse his lender account, as we are tracking the ammount of funds that he owees us and this ammount is includeded in the score calculation
      if(usedAccountsNullifiers[userOwnedAccountsNullifiers[0]]){
        // check if the nullifier was used by the lender account or as one of the ownedAccounts
        require(storedCreditScoresNumber[userOwnedAccountsNullifiers[0]] > 0 , "Provided User lender account is already in use by some other lender account.");
        
      }

      // storing the lender accounts nullifier
      usedAccountsNullifiers[userOwnedAccountsNullifiers[0]] = true;
      ownedAccountNullifiers[userOwnedAccountsNullifiers[0]][block.number].push(userOwnedAccountsNullifiers[0]);
      storedCreditScoresNumber[userOwnedAccountsNullifiers[0]]++;



      // iteratitng over provided nullfiers checking if they were not already in use and storing them
      for (uint i = 1 ; i < userOwnedAccountsNullifiers.length ; i++){
       require(usedAccountsNullifiers[userOwnedAccountsNullifiers[i]] == false, 'User trys to use ethAccount for his maxcredit score calculation, that is already in use.');
        // storing the nullifier
        usedAccountsNullifiers[userOwnedAccountsNullifiers[i]]= true;

        // storing the accountsNullifier in array of the lender account nullifier  in relation to the blockheight at witch the the creditScore was submited
        ownedAccountNullifiers[userOwnedAccountsNullifiers[0]][block.number].push(userOwnedAccountsNullifiers[i]);
      }
    }

    // NOTE: should be called when the CreditScore gets deleted from the contract
    // it deletes all nullifiers related to the creditScore that was submited at the credtiScoreSubmissionBlock height
    function deleteNullifiers(uint256 creditScoreSubmisionBlockHeight, bytes32 lenderNullifier ) external {

      require(ownedAccountNullifiers[lenderNullifier][creditScoreSubmisionBlockHeight].length> 0, "CreditScore related nullifiers not found.");

      // Deleting all usedAccountsNullifiers for this CreditScore calcultation
      for (uint256 index = 1; index < ownedAccountNullifiers[lenderNullifier][creditScoreSubmisionBlockHeight].length; index++) {
        delete usedAccountsNullifiers[ownedAccountNullifiers[lenderNullifier][creditScoreSubmisionBlockHeight][index]];     
      }


      // deleting the array of associated nullifiers 
      delete ownedAccountNullifiers[lenderNullifier][creditScoreSubmisionBlockHeight];

      require(storedCreditScoresNumber[lenderNullifier]> 0 , "User did not have any Credit Score");
      storedCreditScoresNumber[lenderNullifier]--;


      // in case where this was the last used user credit score, user can utilize this lending account as ownedAccount in score calculation for different lending account
      if(storedCreditScoresNumber[lenderNullifier]== 0){
        delete usedAccountsNullifiers[lenderNullifier];
        delete storedCreditScoresNumber[lenderNullifier];
      }


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
