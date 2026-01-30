use alloy::primitives::{Address, B256, U256};
use alloy::rpc::types::eth::Log;
use async_trait::async_trait;
use eyre::Result;

pub struct EthereumLog {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Vec<u8>,
}

impl From<Log> for EthereumLog {
    fn from(log: Log) -> Self {
        Self {
            address: log.address(),
            topics: log.topics().to_vec(),
            data: log.data().data.to_vec(),
        }
    }
}

pub struct SwapEventData {
    pub amount0: U256,
    pub amount1: U256,
    pub price: f64,
    pub sender: Address,
    pub recipient: Address,
}

#[async_trait]
pub trait BaseLiquidityPool: Send + Sync {
    /// Parse log and update internal state (e.g. sqrtPriceX96 or reserves). Returns swap data with price.
    fn parse_swap_event_data(&mut self, log: &EthereumLog) -> Result<SwapEventData>;
    fn get_contract_address(&self) -> Address;
    fn get_event_signatures(&self) -> Vec<B256>;
    fn get_name(&self) -> &str;
    fn get_current_price(&self) -> f64;
    fn apply_initial_state(&mut self, result: Vec<u8>) -> Result<()>;
}

pub struct UniswapV3 {
    address: Address,
    token0_decimals: u8,
    token1_decimals: u8,
    sqrt_price_x96: U256,
}

impl UniswapV3 {
    pub fn new(address: Address, token0_decimals: u8, token1_decimals: u8) -> Self {
        Self {
            address,
            token0_decimals,
            token1_decimals,
            sqrt_price_x96: U256::ZERO,
        }
    }

    fn calculate_price(&self, sqrt_price_x96: U256) -> f64 {
        let q96 = U256::from(2).pow(U256::from(96));
        
        // Use floats for the price calculation to avoid overflow issues with U256
        let sqrt_price_f = sqrt_price_x96.to_string().parse::<f64>().unwrap_or(0.0) / 
                          q96.to_string().parse::<f64>().unwrap_or(1.0);
        
        let price = sqrt_price_f * sqrt_price_f;
        let decimal_adjustment = 10f64.powi(self.token0_decimals as i32 - self.token1_decimals as i32);
        
        price * decimal_adjustment
    }
}

#[async_trait]
impl BaseLiquidityPool for UniswapV3 {
    fn parse_swap_event_data(&mut self, log: &EthereumLog) -> Result<SwapEventData> {
        // Swap(address,address,int256,int256,uint160 sqrtPriceX96,uint128,int24) - sender/recipient in topics, rest in data
        if log.data.len() < 160 {
            return Err(eyre::eyre!("UniswapV3 Swap log data too short"));
        }
        let sqrt_price_x96 = U256::from_be_slice(&log.data[64..96]);
        self.sqrt_price_x96 = sqrt_price_x96;
        let price = self.calculate_price(sqrt_price_x96);
        let amount0 = U256::from_be_slice(&log.data[0..32]);
        let amount1 = U256::from_be_slice(&log.data[32..64]);
        let sender = log.topics.get(1).map(|t| Address::from_slice(&t[12..])).unwrap_or_default();
        let recipient = log.topics.get(2).map(|t| Address::from_slice(&t[12..])).unwrap_or_default();
        Ok(SwapEventData {
            amount0,
            amount1,
            price,
            sender,
            recipient,
        })
    }

    fn get_contract_address(&self) -> Address {
        self.address
    }

    fn get_event_signatures(&self) -> Vec<B256> {
        // keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)")
        vec!["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".parse().unwrap()]
    }

    fn get_name(&self) -> &str {
        "Uniswap V3"
    }

    fn get_current_price(&self) -> f64 {
        self.calculate_price(self.sqrt_price_x96)
    }

    fn apply_initial_state(&mut self, result: Vec<u8>) -> Result<()> {
        // Result is the output of slot0()
        if result.len() >= 32 {
            self.sqrt_price_x96 = U256::from_be_slice(&result[0..32]);
        }
        Ok(())
    }
}

pub struct UniswapV2 {
    address: Address,
    token0_decimals: u8,
    token1_decimals: u8,
    reserve0: U256,
    reserve1: U256,
}

impl UniswapV2 {
    pub fn new(address: Address, token0_decimals: u8, token1_decimals: u8) -> Self {
        Self {
            address,
            token0_decimals,
            token1_decimals,
            reserve0: U256::ZERO,
            reserve1: U256::ZERO,
        }
    }

    fn calculate_price(&self, reserve0: U256, reserve1: U256) -> f64 {
        if reserve0.is_zero() {
            return 0.0;
        }
        
        let r0_f = reserve0.to_string().parse::<f64>().unwrap_or(0.0);
        let r1_f = reserve1.to_string().parse::<f64>().unwrap_or(0.0);
        
        let price = r1_f / r0_f;
        let decimal_adjustment = 10f64.powi(self.token0_decimals as i32 - self.token1_decimals as i32);
        
        price * decimal_adjustment
    }
}

#[async_trait]
impl BaseLiquidityPool for UniswapV2 {
    fn parse_swap_event_data(&mut self, log: &EthereumLog) -> Result<SwapEventData> {
        // keccak256("Swap(address,uint256,uint256,uint256,uint256,address)")
        let swap_topic: B256 = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822".parse().unwrap();
        // keccak256("Sync(uint112,uint112)")
        let sync_topic: B256 = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50438f83b4a47a005e0".parse().unwrap();

        if log.topics.is_empty() {
            return Err(eyre::eyre!("Log has no topics"));
        }

        if log.topics[0] == sync_topic {
            // Sync(reserve0, reserve1) - data is 2 * 32 bytes
            if log.data.len() < 64 {
                return Err(eyre::eyre!("UniswapV2 Sync log data too short"));
            }
            self.reserve0 = U256::from_be_slice(&log.data[0..32]);
            self.reserve1 = U256::from_be_slice(&log.data[32..64]);
            let price = self.calculate_price(self.reserve0, self.reserve1);
            let sender = log.topics.get(1).map(|t| Address::from_slice(&t[12..])).unwrap_or_default();
            Ok(SwapEventData {
                amount0: U256::ZERO,
                amount1: U256::ZERO,
                price,
                sender,
                recipient: Address::ZERO,
            })
        } else if log.topics[0] == swap_topic {
            // Swap(amount0In, amount1In, amount0Out, amount1Out) - we don't get new reserves; use current price
            let price = self.calculate_price(self.reserve0, self.reserve1);
            let amount0 = if log.data.len() >= 32 { U256::from_be_slice(&log.data[0..32]) } else { U256::ZERO };
            let amount1 = if log.data.len() >= 64 { U256::from_be_slice(&log.data[32..64]) } else { U256::ZERO };
            let sender = log.topics.get(1).map(|t| Address::from_slice(&t[12..])).unwrap_or_default();
            Ok(SwapEventData {
                amount0,
                amount1,
                price,
                sender,
                recipient: Address::ZERO,
            })
        } else {
            Err(eyre::eyre!("Not a recognized UniswapV2 event"))
        }
    }

    fn get_contract_address(&self) -> Address {
        self.address
    }

    fn get_event_signatures(&self) -> Vec<B256> {
        vec![
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822".parse().unwrap(),
            "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50438f83b4a47a005e0".parse().unwrap(),
        ]
    }

    fn get_name(&self) -> &str {
        "Uniswap V2"
    }

    fn get_current_price(&self) -> f64 {
        self.calculate_price(self.reserve0, self.reserve1)
    }

    fn apply_initial_state(&mut self, result: Vec<u8>) -> Result<()> {
        // Result is from getReserves() -> (uint112, uint112, uint32)
        if result.len() >= 64 {
            self.reserve0 = U256::from_be_slice(&result[0..32]);
            self.reserve1 = U256::from_be_slice(&result[32..64]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    #[test]
    fn test_uniswap_v2_price_calculation() {
        let pool = UniswapV2::new(
            address!("0000000000000000000000000000000000000000"),
            18, // WETH
            6,  // USDC
        );
        
        // 1 ETH = 2000 USDC
        // r0 = 1 * 10^18, r1 = 2000 * 10^6
        let r0 = U256::from(10).pow(U256::from(18));
        let r1 = U256::from(2000) * U256::from(10).pow(U256::from(6));
        
        let price = pool.calculate_price(r0, r1);
        assert!((price - 2000.0).abs() < 1e-6);
    }

    #[test]
    fn test_uniswap_v3_price_calculation() {
        let pool = UniswapV3::new(
            address!("0000000000000000000000000000000000000000"),
            18, // WETH
            6,  // USDC
        );
        
        // 1 ETH = 2000 USDC
        // Price = 2000 / 10^(18-6) = 2000 / 10^12
        // sqrtPriceX96 = sqrt(Price) * 2^96
        let price_raw = 2000.0 / 10f64.powi(12);
        let sqrt_price = price_raw.sqrt();
        let q96 = 2.0f64.powi(96);
        let sqrt_price_x96_f = sqrt_price * q96;
        let sqrt_price_x96 = U256::from_be_slice(&U256::from(sqrt_price_x96_f as u128).to_be_bytes::<32>());

        let price = pool.calculate_price(sqrt_price_x96);
        assert!((price - 2000.0).abs() < 1.0); // Allow some precision loss in float conversion
    }
}
