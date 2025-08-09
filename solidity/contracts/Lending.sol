pragma solidity ^0.8.20;


/// @title Mock lending contract.
/// @notice This contract is used only for test purposes of the merkle proofs fetching.
contract Lending{
  mapping(address => UserHistory) public users;
  uint256 history = 2;
  struct UserHistory{
    uint256 firstInteractionTimestamp;
    uint256 liquidations;
    uint256 succesfullPayments;
    
  }

    constructor(address userAddress , uint256 firstInteractionTimestamp , uint256 liquidations , uint256 succesfullPayments ) {
      users[userAddress] = UserHistory({
        firstInteractionTimestamp : firstInteractionTimestamp,
        liquidations : liquidations,
        succesfullPayments: succesfullPayments
      });

    }
  
    
    // /// @notice Returns the user history.
    function getUsers(address userAddress) public view returns ( UserHistory memory) {
        return users[userAddress];
    }
}
