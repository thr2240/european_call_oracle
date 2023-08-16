use soroban_sdk::{contract, contractimpl, Address, Env, Map, Vec};

use crate::metadata;
use crate::storage_types::{
    Asset, DataKey, PriceData, INSTANCE_BUMP_AMOUNT, TEMPORARY_BUMP_AMOUNT,
};

pub trait OracleTrait {
    fn initialize(env: Env, admin: Address, base: Asset, decimals: u32, resolution: u32);
    fn has_admin(env: Env) -> bool;
    fn write_admin(env: Env, id: Address);
    fn read_admin(env: Env) -> Address;
    fn add_price(env: Env, source: u32, asset: Asset, price: i128);
    //TODO add bulk prices

    /// Remove prices matching the given conditions.
    /// Parameters:
    ///   sources: a list of sources to match when removing prices. If this is an
    ///            empty list, all sources are be matched.
    ///   assets: a list of assets to match when removing prices. If this is an
    ///            empty list, all assets are matched.
    ///   start_timestamp: prices with timestamp higher than or equal to
    ///            start_timestamp will be matched.
    ///   end_timestamp: prices with timestamp lower than or equal to
    ///            end_timestamp will be matched.
    /// Examples:
    ///   To remove all prices from source=1:
    ///     remove_prices(env, &Vec::<Asset>::from_array(&env, [1]), &Vec::<Asset>::new(&env), None, None);
    ///
    ///   To remove all prices of asset `AssetB` from all sources:
    ///     remove_prices(env, &Vec::<u32>::new(&env), &Vec::<Asset>::from_array(&env, [AssetB]), None, None);
    ///
    ///   To remove all prices of asset `AssetB` from source=1:
    ///     remove_prices(env, &Vec::<u32>::from_array(&env, [1]), &Vec::<Asset>::from_array(&env, [AssetB]), None, None);
    ///
    ///   To remove all prices with timestamp higher than `my_timestamp`:
    ///     remove_prices(env, &Vec::<u32>::new(&env), &Vec::<Asset>::new(&env), &my_timestamp, None);
    fn remove_prices(
        env: Env,
        sources: Vec<u32>,
        assets: Vec<Asset>,
        start_timestamp: Option<u64>,
        end_timestamp: Option<u64>,
    );
    fn base(env: Env) -> Asset;
    fn decimals(env: Env) -> u32;
    fn resolution(env: Env) -> u32;
    fn assets(env: Env) -> Vec<Asset>;
    fn sources(env: Env) -> Vec<u32>;
    fn prices(env: Env, asset: Asset, start_timestamp: u64, end_timestamp: u64) -> Vec<PriceData>;
    fn lastprice(env: Env, asset: Asset) -> Option<PriceData>;
    fn lastprices(env: Env, asset: Asset, records: u32) -> Vec<PriceData>;
    fn prices_by_source(
        env: Env,
        source: u32,
        asset: Asset,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Vec<PriceData>;
    fn lastprices_by_source(env: Env, source: u32, asset: Asset, records: u32) -> Vec<PriceData>;
    fn lastprice_by_source(env: Env, source: u32, asset: Asset) -> Option<PriceData>;
}

#[contract]
pub struct Oracle;

#[contractimpl]
impl OracleTrait for Oracle {
    fn initialize(env: Env, admin: Address, base: Asset, decimals: u32, resolution: u32) {
        if metadata::has_admin(&env) {
            panic!("already initialized")
        }

        metadata::write_metadata(&env, &admin, &base, &decimals, &resolution);
        write_prices(&env, &Map::<u32, Map<Asset, Vec<PriceData>>>::new(&env));
    }

    fn has_admin(env: Env) -> bool {
        env.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
        return metadata::has_admin(&env);
    }

    fn write_admin(env: Env, id: Address) {
        env.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
        metadata::write_admin(&env, &id);
    }

    fn read_admin(env: Env) -> Address {
        env.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
        return metadata::read_admin(&env);
    }

    fn add_price(env: Env, source: u32, asset: Asset, price: i128) {
        metadata::read_admin(&env).require_auth();
        env.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
        let mut prices = read_prices(&env);
        let asset_map_option = prices.get(source);
        let mut asset_map;
        match asset_map_option {
            Some(asset_map_result) => asset_map = asset_map_result,
            None => {
                asset_map = Map::<Asset, Vec<PriceData>>::new(&env);
            }
        }
        let price_data_vec_option = asset_map.get(asset.clone());
        let mut price_data_vec;
        match price_data_vec_option {
            Some(price_data_vec_result) => {
                price_data_vec = price_data_vec_result;
            }
            None => {
                price_data_vec = Vec::<PriceData>::new(&env);
            }
        }
        let timestamp = env.ledger().timestamp();
        if price_data_vec.len() >= 10 {
            price_data_vec.pop_front();
        }
        price_data_vec.push_back(PriceData::new(price, timestamp));
        asset_map.set(asset.clone(), price_data_vec);
        prices.set(source, asset_map);
        write_prices(&env, &prices);
    }

    fn remove_prices(
        env: Env,
        sources: Vec<u32>,
        assets: Vec<Asset>,
        start_timestamp: Option<u64>,
        end_timestamp: Option<u64>,
    ) {
        env.storage().instance().bump(INSTANCE_BUMP_AMOUNT);
        return remove_prices(&env, &sources, &assets, &start_timestamp, &end_timestamp);
    }

    fn base(env: Env) -> Asset {
        return metadata::read_base(&env);
    }

    fn decimals(env: Env) -> u32 {
        return metadata::read_decimals(&env);
    }

    fn resolution(env: Env) -> u32 {
        return metadata::read_resolution(&env);
    }

    fn assets(env: Env) -> Vec<Asset> {
        let prices = read_prices(&env);
        let mut assets_map = Map::<Asset, bool>::new(&env);
        for (_, asset_map) in prices.iter() {
            for (asset, _) in asset_map.iter() {
                assets_map.set(asset, true);
            }
        }
        return assets_map.keys();
    }

    fn sources(env: Env) -> Vec<u32> {
        let prices = read_prices(&env);
        return prices.keys();
    }

    fn prices(env: Env, asset: Asset, start_timestamp: u64, end_timestamp: u64) -> Vec<PriceData> {
        return Oracle::prices_by_source(env, 0, asset, start_timestamp, end_timestamp);
    }

    fn lastprice(env: Env, asset: Asset) -> Option<PriceData> {
        return Oracle::lastprice_by_source(env, 0, asset);
    }

    fn lastprices(env: Env, asset: Asset, records: u32) -> Vec<PriceData> {
        return Oracle::lastprices_by_source(env, 0, asset, records);
    }

    fn prices_by_source(
        env: Env,
        source: u32,
        asset: Asset,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Vec<PriceData> {
        let prices = read_prices(&env);
        let mut prices_within_range: Vec<PriceData> = Vec::<PriceData>::new(&env);
        let asset_map_option = prices.get(source);
        match asset_map_option {
            Some(asset_map) => {
                let prices_vec_option = asset_map.get(asset.clone());
                match prices_vec_option {
                    Some(prices_vec) => {
                        for price_data in prices_vec.iter() {
                            if price_data.timestamp >= start_timestamp
                                && price_data.timestamp <= end_timestamp
                            {
                                prices_within_range.push_back(price_data)
                            }
                        }
                    }
                    None => return prices_within_range,
                }
            }
            None => return prices_within_range,
        }
        return prices_within_range;
    }

    fn lastprices_by_source(env: Env, source: u32, asset: Asset, records: u32) -> Vec<PriceData> {
        let prices = read_prices(&env);
        let mut prices_within_range: Vec<PriceData> = Vec::<PriceData>::new(&env);
        let asset_map_option = prices.get(source);
        match asset_map_option {
            Some(asset_map) => {
                let prices_vec_option = asset_map.get(asset.clone());
                match prices_vec_option {
                    Some(prices_vec) => {
                        let starting_index = prices_vec.len().checked_sub(records).unwrap_or(0);
                        for (index_usize, price_data) in prices_vec.iter().enumerate() {
                            let index: u32 = index_usize.try_into().unwrap();
                            if index < starting_index {
                                continue;
                            }
                            prices_within_range.push_back(price_data)
                        }
                    }
                    None => return prices_within_range,
                }
            }
            None => return prices_within_range,
        }
        return prices_within_range;
    }

    fn lastprice_by_source(env: Env, source: u32, asset: Asset) -> Option<PriceData> {
        let prices = Oracle::lastprices_by_source(env, source, asset, 1);
        for price_data in prices.iter() {
            return Some(price_data);
        }
        return None;
    }
}

fn is_u32_in_vec(n: u32, vec: &Vec<u32>) -> bool {
    for item in vec.iter() {
        if item == n {
            return true;
        }
    }
    return false;
}

fn is_asset_in_vec(asset: Asset, vec: &Vec<Asset>) -> bool {
    for item in vec.iter() {
        if item == asset {
            return true;
        }
    }
    return false;
}

pub fn remove_prices(
    env: &Env,
    sources: &Vec<u32>,
    assets: &Vec<Asset>,
    start_timestamp: &Option<u64>,
    end_timestamp: &Option<u64>,
) {
    metadata::read_admin(&env).require_auth();
    let prices = read_prices(env);
    let mut new_prices = Map::<u32, Map<Asset, Vec<PriceData>>>::new(&env);
    let sources_len = sources.len();
    let assets_len = assets.len();
    for (source, asset_map) in prices.iter() {
        if sources_len > 0 && !is_u32_in_vec(source, &sources) {
            new_prices.set(source, asset_map);
            continue;
        }
        let mut new_asset_map = Map::<Asset, Vec<PriceData>>::new(&env);
        for (asset, price_data_vec) in asset_map.iter() {
            if assets_len > 0 && !is_asset_in_vec(asset.clone(), &assets) {
                new_asset_map.set(asset.clone(), price_data_vec);
                continue;
            }
            let mut new_price_data_vec = Vec::<PriceData>::new(&env);
            for price_data in price_data_vec.iter() {
                match start_timestamp {
                    Some(t) => {
                        if *t < price_data.timestamp {
                            new_price_data_vec.push_back(price_data);
                            continue;
                        }
                    }
                    None => {}
                }
                match end_timestamp {
                    Some(t) => {
                        if *t > price_data.timestamp {
                            new_price_data_vec.push_back(price_data);
                            continue;
                        }
                    }
                    None => {}
                }
            }
            if new_price_data_vec.len() > 0 {
                new_asset_map.set(asset.clone(), new_price_data_vec)
            }
        }
        if new_asset_map.keys().len() > 0 {
            new_prices.set(source, new_asset_map);
        }
    }
    write_prices(env, &new_prices);
}

pub fn read_prices(env: &Env) -> Map<u32, Map<Asset, Vec<PriceData>>> {
    let key = DataKey::Prices;
    env.storage().temporary().bump(&key, TEMPORARY_BUMP_AMOUNT);
    return env.storage().temporary().get(&key).unwrap();
}

pub fn write_prices(env: &Env, prices: &Map<u32, Map<Asset, Vec<PriceData>>>) {
    let key = DataKey::Prices;
    env.storage().temporary().bump(&key, TEMPORARY_BUMP_AMOUNT);
    return env.storage().temporary().set(&key, prices);
}
