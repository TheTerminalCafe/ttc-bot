create table ttc_support_tickets(
	incident_id	SERIAL,						--> This holdes the incident ID , it can be reffered later , must be unique
	thread_id bigint unique not null,				--> This holdes the thread ID that the convo is taking place
	user_id bigint not null,				--> user who made the incident . Example 12345678909875
	incident_time timestamp not null,		--> Time stamp when the incident was opened 
	incident_title VARCHAR(128) not null,	--> tittle of the incident , this must be in limited words
	thread_archived bool not null,
	incident_solved bool not null,
	primary key(incident_id)				--> primary key is incident id , it cant be the same and its ta base of this table
);

