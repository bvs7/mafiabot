pub mod api;
pub mod controller;
pub mod lobby;
pub mod subscriber;
pub mod util;

/* API create message response
{
    "meta":{"code":201},
    "response":{
        "message":{
            "id":"173708255356428677",
            "source_guid":"02585275-dc51-4339-bbcc-dd6f4fd93b12",
            "created_at":1737082553,
            "user_id":"43040067",
            "group_id":"105362524",
            "name":"MODERATOR",
            "avatar_url":"https://i.groupme.com/1920x1080.jpeg.247b8c490afd429f88e239a4914a436d",
            "text":"Testing From API!",
            "system":false,
            "attachments":[],
            "favorited_by":[],
            "sender_type":"user",
            "sender_id":"43040067"
        }
    }
}

*/

/*
Message Example:
[

]
"[{\"channel\":\"/user/43040067\",\"clientId\":\"fayeG3DISD3RLD6KOKLG4LJZTFDQD3LWOTW\",\"id\":\"1591d5f8\",\"data\":{\"alert\":\"Brian Scaramella: Test please\",
\"subject\":{\"attachments\":[],\"avatar_url\":\"https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a\",\"created_at\":1736985478,\"deleted_at\":null,
\"deletion_actor\":null,\"group_id\":\"105362524\",\"id\":\"173698547834459350\",\"location\":{\"lat\":\"\",\"lng\":\"\",\"name\":null},\"name\":\"Brian Scaramella\",
\"parent_id\":null,\"picture_url\":null,\"pinned_at\":null,\"pinned_by\":null,\"sender_id\":\"21642197\",\"sender_type\":\"user\",
\"source_guid\":\"android-ab187b81-37e3-4d2e-b0c1-9636e1dbe998\",\"system\":false,\"text\":\"Test please\",\"updated_at\":null,\"user_id\":\"21642197\"},
\"type\":\"line.create\",\"received_at\":1736985478000}}]"

*/

/* Message "Check" in Main Chat from Brian
Json: Array [
    Object {
        "channel": String("/user/43040067"),
        "clientId": String("fayeF34IPC2IHND7HULV5WAZW2EZNKBMKVO"),
        "data": Object {
            "alert": String("Brian \"Testing\" Scaramella: Test"),
            "received_at": Number(1737048635000),
            "subject": Object {
                "attachments": Array [],
                "avatar_url": String("https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a"),
                "created_at": Number(1737048634),
                "deleted_at": Null,
                "deletion_actor": Null,
                "group_id": String("105362524"),
                "id": String("173704863488783901"),
                "location": Object {
                    "lat": String(""),
                    "lng": String(""),
                    "name": Null,
                },
                "name": String("Brian \"Testing\" Scaramella"),
                "parent_id": Null,
                "picture_url": Null,
                "pinned_at": Null,
                "pinned_by": Null,
                "sender_id": String("21642197"),
                "sender_type": String("user"),
                "source_guid": String("android-bbfe3445-d65e-4352-a613-19df30dede79"),
                "system": Bool(false),
                "text": String("Test"),
                "updated_at": Null,
                "user_id": String("21642197"),
            },
            "type": String("line.create"),
        },
        "id": String("16122cff"),
    },
]
*/

/* Message "/vote @Brian" in Main Chat from Moderator
Json: Array [
    Object {
        "channel": String("/user/43040067"),
        "clientId": String("fayeH7MREDZPQTEG2VHXK2UEDRPO6UR2QP2"),
        "data": Object {
            "alert": String("MODERATOR: /vote @Brian \"Testing\" Scaramella "),
            "received_at": Number(1737048690000),
            "subject": Object {
                "attachments": Array [
                    Object {
                        "loci": Array [
                            Array [
                                Number(6),
                                Number(27),
                            ],
                        ],
                        "type": String("mentions"),
                        "user_ids": Array [
                            Number(21642197),
                        ],
                    },
                ],
                "avatar_url": String("https://i.groupme.com/1920x1080.jpeg.247b8c490afd429f88e239a4914a436d"),
                "created_at": Number(1737048690),
                "deleted_at": Null,
                "deletion_actor": Null,
                "group_id": String("105362524"),
                "id": String("173704869054202031"),
                "location": Object {
                    "lat": String(""),
                    "lng": String(""),
                    "name": Null,
                },
                "name": String("MODERATOR"),
                "parent_id": Null,
                "picture_url": Null,
                "pinned_at": Null,
                "pinned_by": Null,
                "sender_id": String("43040067"),
                "sender_type": String("user"),
                "source_guid": String("ba9a45dff754aa1bcb060a1eb7760135"),
                "system": Bool(false),
                "text": String("/vote @Brian \"Testing\" Scaramella "),
                "updated_at": Null,
                "user_id": String("43040067"),
            },
            "type": String("line.create"),
        },
        "id": String("17d46b97"),
    },
] */
/* DM from Brian to Moderator "Test DM"
Json: Array [
    Object {
        "channel": String("/user/43040067"),
        "clientId": String("fayeLJBD7D2WAWVUDKBKUQEWQ3N4POYHRWH"),
        "data": Object {
            "alert": String("Brian Scaramella: Test DM"),
            "received_at": Number(1737048767000),
            "subject": Object {
                "attachments": Array [],
                "avatar_url": String("https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a"),
                "chat_id": String("21642197+43040067"),
                "created_at": Number(1737048766),
                "favorited_by": Array [],
                "id": String("173704876681069898"),
                "location": Object {
                    "lat": String(""),
                    "lng": String(""),
                    "name": Null,
                },
                "name": String("Brian Scaramella"),
                "picture_url": Null,
                "recipient_id": String("43040067"),
                "sender_id": String("21642197"),
                "sender_type": String("user"),
                "source_guid": String("android-c6edbdaa-a2c0-4ac9-a37b-975de0b73a8b"),
                "text": String("Test DM"),
                "user_id": String("21642197"),
            },
            "type": String("direct_message.create"),
        },
        "id": String("16124d04"),
    },
] */
