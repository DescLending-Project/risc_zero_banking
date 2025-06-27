pragma solidity ^0.8.20;


/// @title A starter application using RISC Zero.
/// @notice This basic application holds a number, guaranteed to be even.
/// @dev This contract demonstrates one pattern for offloading the computation of an expensive
///      or difficult to implement function to a RISC Zero guest running on the zkVM.
contract Lending{
  struct UserHistory{
    uint256 firstInteractionTimestamp;
    uint256 liquidations;
    uint256 succesfullPayments;
    
  }
  mapping(address => UserHistory) public users;
  uint256 history = 2;

    //
    // /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
    constructor(address userAddress , uint256 firstInteractionTimestamp , uint256 liquidations , uint256 succesfullPayments) {
      users[userAddress] = UserHistory({
        firstInteractionTimestamp : firstInteractionTimestamp,
        liquidations : liquidations,
        succesfullPayments: succesfullPayments
      });

    }
  
    
    //
    // /// @notice Set the even number stored on the contract. Requires a RISC Zero proof that the number is even.
    // function setHistory( uint256 liquidations, uint256 succesfull_payments) public {
    //     // Construct the expected journal data. Verify will fail if journal does not match.
    //
    // }
    //
    // /// @notice Returns the number stored.
    function getUsers(address userAddress) public view returns ( UserHistory memory) {
        return users[userAddress];
    }
}
