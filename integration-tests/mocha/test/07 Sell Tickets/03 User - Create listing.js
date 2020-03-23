const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/listings';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{user_token}}'))

        .send(pm.substitute(request_body));
};

let requestBody = `{ "title": "My listing", 
"asking_price_in_cents": 100,
        "items": [
            {
                "ticket_type_id":
                "{{my_tickets_ticket_type_id}}",
                "quantity": 1
                
            }
        ]}`;


describe('User - create listing', function () {
    before(async function () {
        this.timeout(100000);
        response = await post(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);
    });

    after(async function () {
        // add after methods
        let json = JSON.parse(responseBody);
        pm.environment.set("last_listing_id", json.id);


    });

    it("should be 200", function () {
        expect(response.status).to.equal(200);
    })




});

            
