spec diem_framework::resource_account {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec create_resource_account(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
    ) {
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
    }

    spec create_resource_account_and_fund(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
        fund_amount: u64,
    ) {
        use diem_framework::diem_account;
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        let coin_store_resource = global<coin::CoinStore<DiemCoin>>(resource_addr);

        include diem_account::WithdrawAbortsIf<DiemCoin>{from: origin, amount: fund_amount};
        include diem_account::GuidAbortsIf<DiemCoin>{to: resource_addr};
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;

        //coin property
        aborts_if coin::is_account_registered<DiemCoin>(resource_addr) && coin_store_resource.frozen;
    }

    spec create_resource_account_and_publish_package(
        origin: &signer,
        seed: vector<u8>,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        pragma verify = false;
        //TODO: Loop in code.spec
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        let optional_auth_key = ZERO_AUTH_KEY;
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
    }

    spec rotate_account_authentication_key_and_store_capability(
        origin: &signer,
        resource: signer,
        resource_signer_cap: account::SignerCapability,
        optional_auth_key: vector<u8>,
    ) {
        let resource_addr = signer::address_of(resource);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf;
    }

    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf {
        use diem_framework::account::{Account};
        origin: signer;
        resource_addr: address;
        optional_auth_key: vector<u8>;

        let source_addr = signer::address_of(origin);
        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;

        aborts_if get && !exists<Account>(source_addr);
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && !(exists<Account>(resource_addr) && len(global<Account>(source_addr).authentication_key) == 32);
        aborts_if !get && !(exists<Account>(resource_addr) && len(optional_auth_key) == 32);
    }

    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit {
        source_addr: address;
        optional_auth_key: vector<u8>;
        resource_addr: address;

        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;
        let account = global<account::Account>(source_addr);

        requires source_addr != resource_addr;

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include account::exists_at(resource_addr) ==> account::CreateResourceAccountAbortsIf;
        include !account::exists_at(resource_addr) ==> account::CreateAccountAbortsIf {addr: resource_addr};

        aborts_if get && !exists<account::Account>(source_addr);
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && len(global<account::Account>(source_addr).authentication_key) != 32;
        aborts_if !get && len(optional_auth_key) != 32;
    }

    spec retrieve_resource_account_cap(
        resource: &signer,
        source_addr: address,
    ) : account::SignerCapability  {
        aborts_if !exists<Container>(source_addr);
        let resource_addr = signer::address_of(resource);

        let container = borrow_global_mut<Container>(source_addr);
        aborts_if !simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if !exists<account::Account>(resource_addr);
    }
}
