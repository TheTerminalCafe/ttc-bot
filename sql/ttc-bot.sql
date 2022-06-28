DROP TABLE IF EXISTS ttc_support_tickets;
CREATE TABLE ttc_support_tickets(
	incident_id	SERIAL,						--> This holdes the incident ID , it can be reffered later , must be unique
	thread_id BIGINT UNIQUE NOT NULL,		--> This holdes the thread ID that the convo is taking place
	user_id BIGINT NOT NULL,				--> user who made the incident . Example 12345678909875
	incident_time TIMESTAMPTZ NOT NULL,		--> Time stamp when the incident was opened 
	incident_title VARCHAR(128) NOT NULL,	--> title of the incident , this must be in limited words
	incident_solved BOOL NOT NULL,			--> boolean for checking if ticket has been solved
	unarchivals SMALLINT NOT NULL,
	PRIMARY KEY(incident_id)				--> primary key is incident id , it cant be the same and its ta base of this table
);

DROP TABLE IF EXISTS ttc_message_cache;
CREATE TABLE ttc_message_cache(
	id SERIAL,
	message_id BIGINT,
	channel_id BIGINT,
	user_id BIGINT,
	message_time TIMESTAMPTZ,
	content VARCHAR(4000),
	attachments VARCHAR(2000),
	PRIMARY KEY(id)
);

DO $$
DECLARE
   counter INT := 0;
BEGIN
	WHILE counter < 500 LOOP
		counter := counter + 1;
		INSERT INTO ttc_message_cache DEFAULT VALUES;
	END LOOP;
END$$;


DROP TABLE IF EXISTS ttc_conveyance_state;
CREATE TABLE ttc_conveyance_state(
	current_id INT NOT NULL
);

INSERT INTO ttc_conveyance_state (current_id) VALUES(0);

DROP TABLE IF EXISTS ttc_config;
CREATE TABLE ttc_config(
	support_channel BIGINT NOT NULL,
	conveyance_channels BIGINT[] NOT NULL,
	conveyance_blacklisted_channels BIGINT[] NOT NULL,
	welcome_channel BIGINT NOT NULL	,
	verified_role BIGINT NOT NULL,
	moderator_role BIGINT NOT NULL,
	welcome_messages VARCHAR(100)[] NOT NULL
);

DROP TABLE IF EXISTS ttc_bad_words;
CREATE TABLE ttc_bad_words(
    id SERIAL,
    word VARCHAR(4000) NOT NULL,
    PRIMARY KEY(id)
);

DROP TABLE IF EXISTS ttc_webhooks;
CREATE TABLE ttc_webhooks(
	channel_id BIGINT NOT NULL UNIQUE,
	webhook_url VARCHAR NOT NULL,
	PRIMARY KEY(channel_id)
);

DROP TABLE IF EXISTS ttc_easter_egg_gifs;
CREATE TABLE ttc_easter_egg_gifs(
	id SERIAL,
	content VARCHAR(240) NOT NULL,
	PRIMARY KEY(id)
);
