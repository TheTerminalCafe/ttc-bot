-- public.ttc_bad_words definition

-- Drop table

-- DROP TABLE ttc_bad_words;

CREATE TABLE ttc_bad_words (
	id serial4 NOT NULL,
	word varchar(4000) NOT NULL,
	CONSTRAINT ttc_bad_words_pkey PRIMARY KEY (id)
);


-- public.ttc_config_properties definition

-- Drop table

-- DROP TABLE ttc_config_properties;

CREATE TABLE ttc_config_properties (
	id serial4 NOT NULL,
	support_channel int8 NOT NULL,
	welcome_channel int8 NOT NULL,
	verified_role int8 NOT NULL,
	moderator_role int8 NOT NULL,
	CONSTRAINT ttc_config_properties_pk PRIMARY KEY (id)
);


-- public.ttc_conveyance_blacklist_channel definition

-- Drop table

-- DROP TABLE ttc_conveyance_blacklist_channel;

CREATE TABLE ttc_conveyance_blacklist_channel (
	id serial4 NOT NULL,
	channel_id int8 NOT NULL,
	CONSTRAINT ttc_conveyance_blacklist_channel_pk PRIMARY KEY (id)
);


-- public.ttc_conveyance_channel definition

-- Drop table

-- DROP TABLE ttc_conveyance_channel;

CREATE TABLE ttc_conveyance_channel (
	id serial4 NOT NULL,
	channel_id int8 NOT NULL,
	CONSTRAINT ttc_conveyance_channel_pk PRIMARY KEY (id)
);


-- public.ttc_conveyance_state definition

-- Drop table

-- DROP TABLE ttc_conveyance_state;

CREATE TABLE ttc_conveyance_state (
	id serial4 NOT NULL,
	current_message_id int4 NOT NULL,
	CONSTRAINT ttc_conveyance_state_pk PRIMARY KEY (id)
);


-- public.ttc_counted_emoji_name definition

-- Drop table

-- DROP TABLE ttc_counted_emoji_name;

CREATE TABLE ttc_counted_emoji_name (
	id serial4 NOT NULL,
	"name" varchar NOT NULL,
	CONSTRAINT ttc_counted_emoji_names_pk PRIMARY KEY (id)
);


-- public.ttc_easter_egg_gifs definition

-- Drop table

-- DROP TABLE ttc_easter_egg_gifs;

CREATE TABLE ttc_easter_egg_gifs (
	id serial4 NOT NULL,
	"content" varchar(2000) NOT NULL,
	CONSTRAINT ttc_easter_egg_gifs_pkey PRIMARY KEY (id)
);


-- public.ttc_message_cache definition

-- Drop table

-- DROP TABLE ttc_message_cache;

CREATE TABLE ttc_message_cache (
	id serial4 NOT NULL,
	message_id int8 NULL,
	channel_id int8 NULL,
	user_id int8 NULL,
	message_time timestamptz NULL,
	"content" varchar(4000) NULL,
	attachments varchar(2000) NULL,
	CONSTRAINT ttc_message_cache_pkey PRIMARY KEY (id)
);


-- public.ttc_selfroles definition

-- Drop table

-- DROP TABLE ttc_selfroles;

CREATE TABLE ttc_selfroles (
	id serial4 NOT NULL,
	role_id int8 NOT NULL,
	emoji_name varchar NULL,
	CONSTRAINT ttc_selfroles_pk PRIMARY KEY (id)
);


-- public.ttc_support_tickets definition

-- Drop table

-- DROP TABLE ttc_support_tickets;

CREATE TABLE ttc_support_tickets (
	incident_id serial4 NOT NULL,
	thread_id int8 NOT NULL,
	user_id int8 NOT NULL,
	incident_time timestamptz NOT NULL,
	incident_title varchar(128) NOT NULL,
	incident_solved bool NOT NULL,
	unarchivals int2 NOT NULL,
	CONSTRAINT ttc_support_tickets_pkey PRIMARY KEY (incident_id),
	CONSTRAINT ttc_support_tickets_thread_id_key UNIQUE (thread_id)
);


-- public.ttc_webhooks definition

-- Drop table

-- DROP TABLE ttc_webhooks;

CREATE TABLE ttc_webhooks (
	channel_id serial4 NOT NULL,
	webhook_url varchar NOT NULL,
	CONSTRAINT ttc_webhooks_pkey PRIMARY KEY (channel_id)
);


-- public.ttc_welcome_message definition

-- Drop table

-- DROP TABLE ttc_welcome_message;

CREATE TABLE ttc_welcome_message (
	id serial4 NOT NULL,
	welcome_message varchar NOT NULL,
	CONSTRAINT ttc_welcome_messages_pk PRIMARY KEY (id)
);


-- public.ttc_config definition

-- Drop table

-- DROP TABLE ttc_config;

CREATE TABLE ttc_config (
	id serial4 NOT NULL,
	conveyance_id serial4 NOT NULL,
	conveyance_blacklist_id serial4 NOT NULL,
	welcome_message_id serial4 NOT NULL,
	config_properties_id serial4 NOT NULL,
	counted_emoji_name_id serial4 NOT NULL,
	CONSTRAINT ttc_config_pk PRIMARY KEY (id),
	CONSTRAINT fk_config_conveyance FOREIGN KEY (conveyance_id) REFERENCES ttc_conveyance_channel(id),
	CONSTRAINT fk_config_conveyance_blacklist FOREIGN KEY (conveyance_blacklist_id) REFERENCES ttc_conveyance_blacklist_channel(id),
	CONSTRAINT fk_config_properties FOREIGN KEY (config_properties_id) REFERENCES ttc_config_properties(id),
	CONSTRAINT fk_config_welcome FOREIGN KEY (welcome_message_id) REFERENCES ttc_welcome_message(id),
	CONSTRAINT fk_counted_emoji_name FOREIGN KEY (counted_emoji_name_id) REFERENCES ttc_counted_emoji_name(id)
);

-- public.ttc_emoji_cache definition

-- Drop table

-- DROP TABLE ttc_emoji_cache;

CREATE TABLE ttc_emoji_cache (
	user_id int8 NOT NULL,
	emoji_name varchar(32) NOT NULL,
	emoji_count int8 NOT NULL,
	CONSTRAINT ttc_emoji_cache_pkey PRIMARY KEY (user_id, emoji_name)
);


-- public.ttc_emoji_cache_channels definition

-- Drop table

-- DROP TABLE ttc_emoji_cache_channels;

CREATE TABLE ttc_emoji_cache_channels (
	channel_id int8 NOT NULL,
	message_id int8 NOT NULL,
	timestamp_unix int8 NOT NULL,
	CONSTRAINT ttc_emoji_cache_channels_pkey PRIMARY KEY (channel_id)
);


-- public.ttc_emoji_cache_messages definition

-- Drop table

-- DROP TABLE ttc_emoji_cache_messages;

CREATE TABLE ttc_emoji_cache_messages (
	user_id int8 NOT NULL,
	num_messages int8 NOT NULL,
	CONSTRAINT ttc_emoji_cache_messages_pkey PRIMARY KEY (user_id)
);
