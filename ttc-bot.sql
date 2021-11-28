create table ttc_support_tickets(
	incident_id	int,						--> This holdes the incident ID , it can be reffered later , must be unique
	thread_id bigint unique,				--> This holdes the thread ID that the convo is taking place
	incident_time timestamp not null,		--> Time stamp when the incident was opened 
	incident_status VARCHAR(8),				--> current status of the incident. can be [open,hold,closed]
	user_id bigint not null,				--> user who made the incident . Example 12345678909875
	incident_title VARCHAR(128) not null,	--> tittle of the incident , this must be in limited words
	incident_type VARCHAR(32) not null,		--> type of the icnident . Ex OS,Hardware etc
	primary key(incident_id)				--> primary key is incident id , it cant be the same and its ta base of this table
);

