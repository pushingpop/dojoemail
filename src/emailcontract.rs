#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct EmailMessage<M: ManagedTypeApi> {
    pub from: ManagedAddress<M>,
    pub to: ManagedAddress<M>,
    pub subject: ManagedBuffer<M>,
    pub body: ManagedBuffer<M>,
    pub timestamp: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct User<M: ManagedTypeApi> {
  pub alias: ManagedBuffer<M>,
  pub address: ManagedAddress<M>
}

#[multiversx_sc::contract]
pub trait EmailContract {
    #[init]
    fn init(&self) {
        // Inicializa o contrato com o endereço do criador como proprietário
        self.owner().set(self.blockchain().get_caller());
    }

    // Armazena uma nova mensagem de e-mail
    #[endpoint]
    fn store_email(
        &self,
        from: ManagedAddress,
        to: ManagedAddress,
        subject: ManagedBuffer,
        body: ManagedBuffer,
    ) {
        let email = EmailMessage {
            from,
            to,
            subject,
            body,
            timestamp: self.blockchain().get_block_timestamp(),
        };

        let email_id = self.email_count().get();
        self.emails(email_id).set(email);
        self.email_count().set(email_id + 1);

        self.email_stored_event(email_id);
    }

    #[endpoint]
    fn store_email_by_alias(
        &self,
        from: ManagedAddress,
        to: ManagedBuffer,
        subject: ManagedBuffer,
        body: ManagedBuffer,
    ) {
        let address = self.get_address(to);

        let email = EmailMessage {
            from,
            to: address,
            subject,
            body,
            timestamp: self.blockchain().get_block_timestamp(),
        };

        let email_id = self.email_count().get();
        self.emails(email_id).set(email);
        self.email_count().set(email_id + 1);

        self.email_stored_event(email_id);
    }

    #[endpoint]
    fn register(
        &self,
        alias: ManagedBuffer,
    ) {
      let user = User {
        alias: alias.clone(),
        address: self.blockchain().get_caller()
      };

      // Armazena o usuário no mapeamento
      self.users(alias).set(user);
    }

    fn get_address(&self, alias: ManagedBuffer) -> ManagedAddress<Self::Api> {
      // Recupera o endereço do usuário pelo alias
      let user = self.users(alias).get();
      require!(user.address != ManagedAddress::zero(), "Usuário não encontrado");

      // Retorna o endereço do usuário
      user.address
    }

    // Recupera uma mensagem de e-mail específica pelo ID
    #[view]
    fn get_email(&self, email_id: u64) -> EmailMessage<Self::Api> {
        // Verifica se o chamador é o proprietário
        self.require_owner();

        // Verifica se o ID é válido
        require!(email_id < self.email_count().get(), "Email ID inválido");

        // Retorna a mensagem de e-mail
        self.emails(email_id).get()
    }

    // Recupera todos os emails enviados para um determinado endereço
    #[view]
    fn get_emails_by_recipient(&self, recipient: ManagedAddress) -> MultiValueEncoded<EmailMessage<Self::Api>> {
        // Verifica se o chamador é o proprietário ou o próprio recebedor
        let caller = self.blockchain().get_caller();
        require!(
            caller == self.owner().get() || caller == recipient,
            "Apenas o proprietário ou o recebedor pode visualizar estes emails"
        );

        let email_count = self.email_count().get();
        let mut result = MultiValueEncoded::new();

        // Itera por todos os emails e filtra pelos recebedores
        for i in 0..email_count {
            let email = self.emails(i).get();
            if email.to == recipient {
                result.push(email);
            }
        }

        result
    }

    // Recupera todas as mensagens de e-mail
    #[view]
    fn get_all_emails(&self) -> MultiValueEncoded<EmailMessage<Self::Api>> {
        // Verifica se o chamador é o proprietário
        self.require_owner();

        let email_count = self.email_count().get();
        let mut result = MultiValueEncoded::new();

        for i in 0..email_count {
            result.push(self.emails(i).get());
        }

        result
    }

    // Recupera o número total de e-mails armazenados
    #[view]
    fn get_email_count(&self) -> u64 {
        self.email_count().get()
    }

    // Verifica se o chamador é o proprietário
    fn require_owner(&self) {
        require!(
            self.blockchain().get_caller() == self.owner().get(),
            "Apenas o proprietário pode executar esta função"
        );
    }

    // Eventos
    #[event("email_stored")]
    fn email_stored_event(&self, #[indexed] email_id: u64);

    // Armazenamento
    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("emails")]
    fn emails(&self, id: u64) -> SingleValueMapper<EmailMessage<Self::Api>>;

    #[storage_mapper("emailCount")]
    fn email_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("users")]
    fn users(&self, alias: ManagedBuffer) -> SingleValueMapper<User<Self::Api>>;
}