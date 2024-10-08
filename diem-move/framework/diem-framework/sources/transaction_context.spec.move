spec diem_framework::transaction_context {
    spec get_script_hash(): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures result == spec_get_script_hash();
    }
    spec fun spec_get_script_hash(): vector<u8>;
    spec get_txn_hash(): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures result == spec_get_txn_hash();
    }
    spec fun spec_get_txn_hash(): vector<u8>;
    spec generate_unique_address(): address {
        pragma opaque;
    }
}
