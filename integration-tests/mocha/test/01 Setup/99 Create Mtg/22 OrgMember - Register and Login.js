const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../../pm');const debug=require('debug');var log = debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/users';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))


        .set('Accept', 'application/json')
        .send();
};

let requestBody = `{
	"first_name":"Org",
	"last_name":"Member",
	"email":"{{mtg_last_org_member_email}}",
	"phone":"555",
	"password": "itsasecret"
}`;


describe('OrgMember - Register and Login', function () {
    before(async function () {
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

        pm.environment.set("mtg_org_member_token", json.access_token);

    });

    it("should be 201", function () {
        expect(response.status).to.equal(201);
    })
    it("should have token response", function () {
        let json = JSON.parse(responseBody);
        expect(json).to.have.property("access_token");
        expect(json).to.have.property("refresh_token");
    });


});

            
