const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');
const debug = require('debug');
var log = debug('bn-api');
const events = require('../../helpers/events');
const cart = require('../../helpers/cart');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/tickets?query=';


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

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Authorization', pm.substitute('Bearer {{user_token}}'))

        .set('Accept', 'application/json')
        .send();
};

let requestBody = ``;


describe('User - Create with order', function () {
    before(async function () {
        this.timeout(100000);
        let event = await events.create("__get_tickets");
        await cart.createPaid(event, 6);
        response = await get(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);
    });


    it("should be 200", function () {
        expect(response.status).to.equal(200);
        let json = JSON.parse(responseBody);
    console.log(responseBody);
        pm.environment.set("my_tickets_ticket_type_id", json.data[0][1][0].collectible_id);
    })


});

            
