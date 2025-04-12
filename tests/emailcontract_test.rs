use emailcontract::*;
use multiversx_sc::types::Address;
use multiversx_sc::contract_base::ContractBase;
use multiversx_sc_scenario::{
    managed_address, managed_buffer, rust_biguint, testing_framework::*, DebugApi,
};

const WASM_PATH: &str = "output/email-contract.wasm";

struct ContractSetup<ContractObjBuilder>
where
    ContractObjBuilder: 'static + Copy + Fn() -> emailcontract::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub contract_wrapper: ContractObjWrapper<emailcontract::ContractObj<DebugApi>, ContractObjBuilder>,
}

fn setup_contract<ContractObjBuilder>(
    cf_builder: ContractObjBuilder,
) -> ContractSetup<ContractObjBuilder>
where
    ContractObjBuilder: 'static + Copy + Fn() -> emailcontract::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_address = blockchain_wrapper.create_user_account(&rust_zero);
    
    let contract_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        cf_builder,
        WASM_PATH,
    );

    // Set up the contract explicitly with the owner
    blockchain_wrapper
        .execute_tx(&owner_address, &contract_wrapper, &rust_zero, |sc| {
            sc.init();
            // Make sure the owner is explicitly set
            let caller = sc.blockchain().get_caller();
            sc.owner().set(caller);
        })
        .assert_ok();

    ContractSetup {
        blockchain_wrapper,
        owner_address,
        contract_wrapper,
    }
}

#[test]
fn test_store_and_retrieve_email() {
    let mut setup = setup_contract(emailcontract::contract_obj);
    let owner_address = setup.owner_address.clone();

    // Armazenar um e-mail
    setup
        .blockchain_wrapper
        .execute_tx(&owner_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            let from = managed_address!(&owner_address);
            let to = managed_address!(&owner_address);
            let subject = managed_buffer!(b"Test Subject");
            let body = managed_buffer!(b"This is a test email body");

            sc.store_email(from, to, subject, body);

            // Verificar contagem de e-mails
            assert_eq!(sc.get_email_count(), 1);
        })
        .assert_ok();

    // Explicitly set the blockchain caller for queries too
    // No need to explicitly set the caller for queries
    // Recuperar o e-mail
    setup
        .blockchain_wrapper
        .execute_tx(&owner_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            let email = sc.get_email(0);
            
            assert_eq!(email.from.to_address(), owner_address);
            assert_eq!(email.to.to_address(), owner_address);
            assert_eq!(email.subject.to_boxed_bytes().as_slice(), b"Test Subject");
            assert_eq!(email.body.to_boxed_bytes().as_slice(), b"This is a test email body");
        })
        .assert_ok();
}

#[test]
fn test_get_emails_by_recipient() {
    let mut setup = setup_contract(emailcontract::contract_obj);
    let owner_address = setup.owner_address.clone();
    
    // Criar um segundo endereço para teste
    let recipient_address = setup.blockchain_wrapper.create_user_account(&rust_biguint!(0));
    
    // Armazenar emails com diferentes destinatários
    setup
        .blockchain_wrapper
        .execute_tx(&owner_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            // Email 1: owner -> recipient
            sc.store_email(
                managed_address!(&owner_address),
                managed_address!(&recipient_address),
                managed_buffer!(b"Email para recipient 1"),
                managed_buffer!(b"Corpo do email 1"),
            );
            
            // Email 2: owner -> owner
            sc.store_email(
                managed_address!(&owner_address),
                managed_address!(&owner_address),
                managed_buffer!(b"Email para owner"),
                managed_buffer!(b"Corpo do email para owner"),
            );
            
            // Email 3: owner -> recipient
            sc.store_email(
                managed_address!(&owner_address),
                managed_address!(&recipient_address),
                managed_buffer!(b"Email para recipient 2"),
                managed_buffer!(b"Corpo do email 2"),
            );
            
            // Verificar contagem total de e-mails
            assert_eq!(sc.get_email_count(), 3);
        })
        .assert_ok();
    
    // Teste 1: O proprietário pode visualizar os emails de qualquer destinatário
    setup
        .blockchain_wrapper
        .execute_tx(&owner_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            // Buscar emails do recipient
            let recipient_emails = sc.get_emails_by_recipient(managed_address!(&recipient_address));
            
            // Deve retornar 2 emails
            assert_eq!(recipient_emails.len(), 2);
            
            // Iterar sobre os elementos diretamente
            let mut recipient_emails_iter = recipient_emails.into_iter();
            
            // Verificar conteúdo do primeiro email para recipient
            if let Some(first_email) = recipient_emails_iter.next() {
                assert_eq!(first_email.from.to_address(), owner_address);
                assert_eq!(first_email.to.to_address(), recipient_address);
                assert_eq!(first_email.subject.to_boxed_bytes().as_slice(), b"Email para recipient 1");
            }
            
            // Verificar conteúdo do segundo email para recipient
            if let Some(second_email) = recipient_emails_iter.next() {
                assert_eq!(second_email.from.to_address(), owner_address);
                assert_eq!(second_email.to.to_address(), recipient_address);
            }
            
            // Buscar emails do owner
            let owner_emails = sc.get_emails_by_recipient(managed_address!(&owner_address));
            
            // Deve retornar 1 email
            assert_eq!(owner_emails.len(), 1);
            // Iterar sobre os elementos diretamente
            let mut owner_emails_iter = owner_emails.into_iter();
            
            // Verificar conteúdo do email para owner
            if let Some(owner_email) = owner_emails_iter.next() {
                // Verificar conteúdo do email para owner
                assert_eq!(owner_email.from.to_address(), owner_address);
            // Obter emails do recipient novamente
            let my_emails = sc.get_emails_by_recipient(managed_address!(&recipient_address));
            
            // Deve retornar 2 emails
            assert_eq!(my_emails.len(), 2);
            
            // Coletar emails em um vetor para testar índices específicos
            let my_emails_vec: Vec<_> = my_emails.into_iter().collect();
            
            // Verificar que são os emails corretos
            assert_eq!(my_emails_vec[0].to.to_address(), recipient_address);
            assert_eq!(my_emails_vec[1].to.to_address(), recipient_address);
        }})
        .assert_ok();
    
    // Teste 3: Um usuário não pode visualizar emails de outro usuário
    setup
        .blockchain_wrapper
        .execute_tx(&recipient_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            // Tentativa de acessar emails do owner deve falhar
            let result = sc.get_emails_by_recipient(managed_address!(&owner_address));
        })
        .assert_user_error("Apenas o proprietário ou o recebedor pode visualizar estes emails");
}

#[test]
fn test_send_email_with_alias() {
    let mut setup = setup_contract(emailcontract::contract_obj);
    let owner_address = setup.owner_address.clone();
    
    // Criar um endereço para o destinatário
    let receiver_address = setup.blockchain_wrapper.create_user_account(&rust_biguint!(0));
    
    // Definir um alias para o remetente
    let sender_alias = b"John Doe";
    
    // Armazenar um e-mail com alias
    setup
        .blockchain_wrapper
        .execute_tx(&owner_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            // Configurar o alias para o proprietário
            sc.register(managed_buffer!(sender_alias));
            
            // Enviar e-mail usando o endereço normal
            sc.store_email(
                managed_address!(&owner_address),
                managed_address!(&receiver_address),
                managed_buffer!(b"Email with alias"),
                managed_buffer!(b"This is a test email sent with an alias"),
            );
            
            // Verificar contagem de e-mails
            assert_eq!(sc.get_email_count(), 1);
            
            // Verificar se o alias foi armazenado corretamente
            // Verificamos o alias pelo storage_mapper 'users'
            let user = sc.users(managed_buffer!(sender_alias)).get();
            assert_eq!(user.alias.to_boxed_bytes().as_slice(), sender_alias);
            assert_eq!(user.address.to_address(), owner_address);
        })
        .assert_ok();
    
    // Verificar se o destinatário recebe o e-mail corretamente com o alias do remetente
    setup
        .blockchain_wrapper
        .execute_tx(&receiver_address, &setup.contract_wrapper, &rust_biguint!(0), |sc| {
            // Recuperar e-mails recebidos
            let received_emails = sc.get_emails_by_recipient(managed_address!(&receiver_address));
            
            // Deve haver 1 e-mail
            assert_eq!(received_emails.len(), 1);
            
            let received_emails_vec: Vec<_> = received_emails.into_iter().collect();
            let email = &received_emails_vec[0];
            
            // Verificar conteúdo do e-mail
            assert_eq!(email.from.to_address(), owner_address);
            assert_eq!(email.to.to_address(), receiver_address);
            assert_eq!(email.subject.to_boxed_bytes().as_slice(), b"Email with alias");
            
            // Verificar alias do remetente - não podemos obter diretamente por email.from
            // Mas podemos verificar que existe um alias registrado com o endereço correto
            let user = sc.users(managed_buffer!(sender_alias)).get();
            assert_eq!(user.address, email.from);
            assert_eq!(user.alias.to_boxed_bytes().as_slice(), sender_alias);
        })
        .assert_ok();
}