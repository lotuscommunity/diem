// :!:>module
/// Mock coin transfer module that invokes governance parameters.
module upgrade_and_govern::transfer {

    use diem_framework::diem_coin::DiemCoin;
    use diem_framework::coin;
    use upgrade_and_govern::parameters;

    public entry fun transfer_octas(
        from: &signer,
        to_1: address,
        to_2: address
    ) {
        let (amount_1, amount_2) = parameters::get_parameters();
        coin::transfer<DiemCoin>(from, to_1, amount_1);
        coin::transfer<DiemCoin>(from, to_2, amount_2);
    }

} // <:!:module
