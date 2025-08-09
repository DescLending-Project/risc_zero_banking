// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import "forge-std/Test.sol";
import {stdToml} from "forge-std/StdToml.sol";
import {RiscZeroCheats} from "risc0/test/RiscZeroCheats.sol";
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {RiscZeroGroth16Verifier} from "risc0/groth16/RiscZeroGroth16Verifier.sol";
import {ControlID} from "risc0/groth16/ControlID.sol";
import {CreditScore} from "../contracts/Creditscore.sol";
import {Lending} from "../contracts/Lending.sol";

contract CreditScoreDeploy is Script, RiscZeroCheats {
    string constant CONFIG_FILE = "deploy/config.toml";
    IRiscZeroVerifier verifier;

    function run() external {
        uint256 chainId = block.chainid;
        console2.log("Deploying CreditScore on ChainID %d", chainId);

        // Read the config profile from the environment variable, or use the default for the chainId.
        // Default is the first profile with a matching chainId field.
        string memory config = vm.readFile(string.concat(vm.projectRoot(), "/", CONFIG_FILE));
        string memory configProfile = vm.envOr("CONFIG_PROFILE", string(""));
        
        if (bytes(configProfile).length == 0) {
            string[] memory profileKeys = vm.parseTomlKeys(config, ".profile");
            for (uint256 i = 0; i < profileKeys.length; i++) {
                if (stdToml.readUint(config, string.concat(".profile.", profileKeys[i], ".chainId")) == chainId) {
                    configProfile = profileKeys[i];
                    break;
                }
            }
        }

        if (bytes(configProfile).length != 0) {
            console2.log("Deploying using config profile:", configProfile);
            string memory configProfileKey = string.concat(".profile.", configProfile);
            address riscZeroVerifierAddress =
                stdToml.readAddress(config, string.concat(configProfileKey, ".riscZeroVerifierAddress"));
            // If set, use the predeployed verifier address found in the config.
            verifier = IRiscZeroVerifier(riscZeroVerifierAddress);
        }

        uint256 deployerKey = uint256(vm.envOr("ETH_WALLET_PRIVATE_KEY", bytes32(0)));
        address deployerAddr = address(0);
        
        if (deployerKey != 0) {
            // Check for conflicts in how the two environment variables are set.
            address envAddr = vm.envOr("ETH_WALLET_ADDRESS", address(0));
            require(
                envAddr == address(0) || envAddr == vm.addr(deployerKey),
                "conflicting settings from ETH_WALLET_PRIVATE_KEY and ETH_WALLET_ADDRESS"
            );
            vm.startBroadcast(deployerKey);
        } else {
            deployerAddr = vm.envAddress("ETH_WALLET_ADDRESS");
            vm.startBroadcast(deployerAddr);
        }

        // CRITICAL: Deploy a fresh verifier that matches RISC0 v2.1.0
        if (address(verifier) == address(0)) {
            console2.log("Deploying fresh RiscZeroGroth16Verifier for v2.1.0...");
            verifier = deployRiscZeroVerifier();
        } else {
            console2.log("Using IRiscZeroVerifier contract deployed at", address(verifier));
        }

        // Deploy mock lending contract 
        address userAddress = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;
        Lending lending = new Lending(userAddress, 1,2,3);
        console2.log("Lending contract deployed at", address(lending));

        // Deploy the CreditScore contract.
        CreditScore creditScore = new CreditScore(verifier);
        console2.log("Deployed CreditScore contract to", address(creditScore));
        console2.log("ImageID constant:", uint256(creditScore.imageId()));

        vm.stopBroadcast();
    }
}
