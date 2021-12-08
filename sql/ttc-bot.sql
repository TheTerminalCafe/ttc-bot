DROP TABLE IF EXISTS ttc_support_tickets;
CREATE TABLE ttc_support_tickets(
	incident_id	SERIAL,						--> This holdes the incident ID , it can be reffered later , must be unique
	thread_id BIGINT UNIQUE NOT NULL,		--> This holdes the thread ID that the convo is taking place
	user_id BIGINT NOT NULL,				--> user who made the incident . Example 12345678909875
	incident_time TIMESTAMPTZ NOT NULL,		--> Time stamp when the incident was opened 
	incident_title VARCHAR(128) NOT NULL,	--> title of the incident , this must be in limited words
	incident_solved BOOL NOT NULL,			--> boolean for checking if ticket has been solved
	PRIMARY KEY(incident_id)				--> primary key is incident id , it cant be the same and its ta base of this table
);

DROP TABLE IF EXISTS ttc_message_cache;
CREATE TABLE ttc_message_cache(
	id SERIAL,
	message_id BIGINT UNIQUE NOT NULL,
	channel_id BIGINT UNIQUE NOT NULL,
	user_id BIGINT UNIQUE NOT NULL,
	message_time TIMESTAMPTZ NOT NULL,
	content VARCHAR(4000),
	attachments VARCHAR(2000),
	PRIMARY KEY(id)
);

DROP TABLE IF EXISTS ttc_conveyance_state;
CREATE TABLE ttc_conveyance_state(
	id SERIAL,
	current_id SMALLINT NOT NULL,
	PRIMARY KEY(id)
);

INSERT INTO ttc_conveyance_state (current_id) VALUES(1);
