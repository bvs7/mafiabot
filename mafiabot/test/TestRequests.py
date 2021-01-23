import requests

requests.post("http://localhost:1121/",json={"text":"/in", "group_id":'test_lobby','sender_id':'1'})
requests.post("http://localhost:1121/",json={"text":"/in", "group_id":'test_lobby','sender_id':'2'})
requests.post("http://localhost:1121/",json={"text":"/in", "group_id":'test_lobby','sender_id':'3'})
requests.post("http://localhost:1121/",json={"text":"/start 1", "group_id":'test_lobby','sender_id':'3'})