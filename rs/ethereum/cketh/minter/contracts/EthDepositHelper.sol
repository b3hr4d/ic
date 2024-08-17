// SPDX-License-Identifier: Apache-2.0

pragma solidity 0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/**
 * @title A combined smart contract for ETH <-> ckETH conversion and ERC20 token deposits.
 * @notice This smart contract deposits incoming ETH to the ckETH minter account, emits deposit events, and helps in depositing ERC20 tokens.
 */
contract CombinedDepositHelper {
    address payable private immutable cketh_minter_main_address;
    address public tokenAddress;

    event ReceivedEth(
        address indexed from,
        uint256 value,
        bytes32 indexed principal
    );

    event TokenDeposited(address indexed from, uint256 amount);

    /**
     * @dev Set cketh_minter_main_address and tokenAddress.
     */
    constructor(address _cketh_minter_main_address, address _tokenAddress) {
        cketh_minter_main_address = payable(_cketh_minter_main_address);
        tokenAddress = _tokenAddress;
    }

    /**
     * @dev Return ckETH minter main address.
     * @return address of ckETH minter main address.
     */
    function getMinterAddress() public view returns (address) {
        return cketh_minter_main_address;
    }

    /**
     * @dev Emits the `ReceivedEth` event if the transfer succeeds.
     */
    function depositEth(bytes32 _principal) public payable {
        emit ReceivedEth(msg.sender, msg.value, _principal);
        cketh_minter_main_address.transfer(msg.value);
    }

    /**
     * @dev Deposits ERC20 tokens.
     * @param amount The amount of tokens to deposit.
     */
    function depositTokens(uint256 amount) public {
        require(
            IERC20(tokenAddress).transferFrom(
                msg.sender,
                address(this),
                amount
            ),
            "Transfer failed"
        );
        emit TokenDeposited(msg.sender, amount);
    }

    /**
     * @dev Withdraws ERC20 tokens.
     * @param amount The amount of tokens to withdraw.
     */
    function withdrawTokens(uint256 amount) public {
        require(
            IERC20(tokenAddress).transfer(msg.sender, amount),
            "Transfer failed"
        );
    }
}
