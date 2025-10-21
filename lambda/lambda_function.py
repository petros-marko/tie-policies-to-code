import json

def lambda_handler(event, context):
    print("Received event: " + json.dumps(event, indent=2))
    
    return {
        'statusCode': 200,
        'headers': {
            'Content-Type': 'application/json'
        },
        'body': json.dumps({
            'message': 'Hello from Lambda!',
            'event': event
        })
    }
