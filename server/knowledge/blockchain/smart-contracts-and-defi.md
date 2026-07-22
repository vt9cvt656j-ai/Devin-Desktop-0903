# Blockchain: Smart Contracts & DeFi

## Solidity Fundamentals (EVM)

### Contract Structure
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract MyToken is ERC20, Ownable, ReentrancyGuard {
    uint256 public constant MAX_SUPPLY = 1_000_000 * 10**18;

    mapping(address => uint256) public stakingBalance;
    mapping(address => uint256) public stakingTimestamp;

    event Staked(address indexed user, uint256 amount);
    event Unstaked(address indexed user, uint256 amount, uint256 reward);

    constructor() ERC20("MyToken", "MTK") Ownable(msg.sender) {
        _mint(msg.sender, MAX_SUPPLY);
    }

    function stake(uint256 amount) external nonReentrant {
        require(amount > 0, "Cannot stake 0");
        require(balanceOf(msg.sender) >= amount, "Insufficient balance");

        _transfer(msg.sender, address(this), amount);
        stakingBalance[msg.sender] += amount;
        stakingTimestamp[msg.sender] = block.timestamp;

        emit Staked(msg.sender, amount);
    }

    function unstake() external nonReentrant {
        uint256 staked = stakingBalance[msg.sender];
        require(staked > 0, "Nothing staked");

        uint256 reward = _calculateReward(msg.sender);
        stakingBalance[msg.sender] = 0;

        _transfer(address(this), msg.sender, staked);
        if (reward > 0) _mint(msg.sender, reward);

        emit Unstaked(msg.sender, staked, reward);
    }

    function _calculateReward(address user) internal view returns (uint256) {
        uint256 duration = block.timestamp - stakingTimestamp[user];
        return (stakingBalance[user] * duration * 10) / (365 days * 100); // 10% APY
    }
}
```

### Security Vulnerabilities & Prevention

| Vulnerability | Prevention |
|--------------|-----------|
| Reentrancy | `nonReentrant` modifier, checks-effects-interactions pattern |
| Integer overflow | Solidity 0.8+ has built-in overflow checks |
| Front-running | Commit-reveal, use private mempool (Flashbots) |
| Oracle manipulation | TWAP oracles, Chainlink, multiple sources |
| Access control | OpenZeppelin `Ownable`/`AccessControl`, multi-sig for admin |
| Unchecked return | Always check `.transfer()` / `.call()` return values |
| Delegatecall | Never to untrusted contracts, storage layout must match |
| Signature replay | Include nonce + chainId + contract address in signed data (EIP-712) |

### Checks-Effects-Interactions Pattern
```solidity
function withdraw(uint256 amount) external nonReentrant {
    // CHECKS
    require(balances[msg.sender] >= amount, "Insufficient");

    // EFFECTS (state changes BEFORE external calls)
    balances[msg.sender] -= amount;

    // INTERACTIONS (external calls LAST)
    (bool success, ) = msg.sender.call{value: amount}("");
    require(success, "Transfer failed");
}
```

## DeFi Patterns

### AMM (Automated Market Maker) — Uniswap V2 Style
```solidity
// Constant product: x * y = k
// Price impact: larger trade → more slippage

function getAmountOut(
    uint256 amountIn,
    uint256 reserveIn,
    uint256 reserveOut
) public pure returns (uint256) {
    uint256 amountInWithFee = amountIn * 997;  // 0.3% fee
    uint256 numerator = amountInWithFee * reserveOut;
    uint256 denominator = (reserveIn * 1000) + amountInWithFee;
    return numerator / denominator;
}

// Slippage protection
function swapExactTokensForTokens(
    uint256 amountIn,
    uint256 amountOutMin,  // ALWAYS set this (slippage tolerance)
    address[] calldata path,
    address to,
    uint256 deadline        // ALWAYS set this (prevents stale txn execution)
) external returns (uint256[] memory amounts);
```

### Flash Loan Pattern
```solidity
// Borrow any amount, use it, return it + fee — all in one transaction
// If not returned → entire transaction reverts

interface IFlashLoanReceiver {
    function executeOperation(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,  // fees to pay back
        address initiator,
        bytes calldata params
    ) external returns (bool);
}

// Use cases: arbitrage, collateral swap, self-liquidation
// Attack vector: price oracle manipulation within single tx
// Defense: use TWAP oracles, not spot price
```

### Chainlink Oracle Integration
```solidity
import "@chainlink/contracts/src/v0.8/interfaces/AggregatorV3Interface.sol";

contract PriceConsumer {
    AggregatorV3Interface internal priceFeed;

    constructor() {
        // ETH/USD on Ethereum mainnet
        priceFeed = AggregatorV3Interface(0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419);
    }

    function getLatestPrice() public view returns (int256, uint256) {
        (
            uint80 roundID,
            int256 price,
            uint256 startedAt,
            uint256 updatedAt,
            uint80 answeredInRound
        ) = priceFeed.latestRoundData();

        // CRITICAL: validate oracle data freshness
        require(updatedAt > block.timestamp - 3600, "Stale price");
        require(price > 0, "Invalid price");
        require(answeredInRound >= roundID, "Stale round");

        return (price, updatedAt);  // price has 8 decimals
    }
}
```

## Web3 Frontend Integration

### Wallet Connection (wagmi + viem)
```typescript
import { createConfig, http } from 'wagmi';
import { mainnet, arbitrum, optimism } from 'wagmi/chains';
import { injected, walletConnect } from 'wagmi/connectors';

const config = createConfig({
  chains: [mainnet, arbitrum, optimism],
  connectors: [
    injected(),
    walletConnect({ projectId: process.env.WC_PROJECT_ID }),
  ],
  transports: {
    [mainnet.id]: http('https://eth.llamarpc.com'),
    [arbitrum.id]: http('https://arb1.arbitrum.io/rpc'),
    [optimism.id]: http('https://mainnet.optimism.io'),
  },
});

// Reading contract state
import { useReadContract } from 'wagmi';
function TokenBalance({ address }) {
  const { data: balance } = useReadContract({
    address: '0x...tokenAddress',
    abi: erc20Abi,
    functionName: 'balanceOf',
    args: [address],
  });
  return <span>{formatUnits(balance, 18)} tokens</span>;
}

// Writing (sending transaction)
import { useWriteContract, useWaitForTransactionReceipt } from 'wagmi';
function StakeButton({ amount }) {
  const { writeContract, data: hash } = useWriteContract();
  const { isSuccess } = useWaitForTransactionReceipt({ hash });

  return (
    <button onClick={() => writeContract({
      address: '0x...stakingContract',
      abi: stakingAbi,
      functionName: 'stake',
      args: [parseUnits(amount, 18)],
    })}>
      {isSuccess ? 'Staked!' : 'Stake'}
    </button>
  );
}
```

## Testing Smart Contracts

### Foundry (Solidity-native testing)
```solidity
// test/MyToken.t.sol
import "forge-std/Test.sol";
import "../src/MyToken.sol";

contract MyTokenTest is Test {
    MyToken token;
    address alice = makeAddr("alice");
    address bob = makeAddr("bob");

    function setUp() public {
        token = new MyToken();
        token.transfer(alice, 1000 ether);
    }

    function testStakeAndUnstake() public {
        vm.startPrank(alice);
        token.stake(500 ether);
        assertEq(token.stakingBalance(alice), 500 ether);

        // Fast-forward 365 days
        vm.warp(block.timestamp + 365 days);
        token.unstake();

        // Should have ~10% reward
        assertGt(token.balanceOf(alice), 1000 ether);
        vm.stopPrank();
    }

    function testCannotStakeZero() public {
        vm.prank(alice);
        vm.expectRevert("Cannot stake 0");
        token.stake(0);
    }

    // Fuzz testing
    function testFuzz_Stake(uint256 amount) public {
        amount = bound(amount, 1, 1000 ether);
        vm.prank(alice);
        token.stake(amount);
        assertEq(token.stakingBalance(alice), amount);
    }
}
```

### Gas Optimization
```
1. Use uint256 over smaller types (EVM operates on 256-bit words)
2. Pack storage variables (multiple small vars in one slot = 32 bytes)
3. Use calldata instead of memory for read-only function params
4. Cache storage reads in local variables (SLOAD = 2100 gas)
5. Use unchecked{} for math where overflow is impossible
6. Use events instead of storage for data only needed off-chain
7. Use immutable/constant for values set once
8. Batch operations (one tx with loop vs many txs)
```

## Common LLM Mistakes in Smart Contracts
```
1. Using transfer() instead of call() (transfer has 2300 gas limit, breaks with receive())
2. Not checking call() return value: (bool success, ) = addr.call{value: x}(""); require(success);
3. Missing nonReentrant on functions with external calls
4. Using block.timestamp for randomness (miner-manipulable)
5. Not validating Chainlink oracle freshness (stale price attacks)
6. Hardcoding gas limits (will break on EVM upgrades)
7. Missing deadline parameter on swap functions
8. Not accounting for fee-on-transfer tokens in DeFi
9. Using tx.origin for auth (phishable via intermediate contract)
10. Forgetting to emit events on state changes
```
