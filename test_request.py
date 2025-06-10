import requests; response = requests.post('http://localhost:3000/api/tasks', json={'title': '测试任务', 'description': '这是一个测试任务', 'completed': False}); print(response.status_code); print(response.text)
